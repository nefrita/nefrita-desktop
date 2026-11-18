# Running a custom PIN server

A Nefrita board unlocks its wallet with a PIN through a PIN server, which never learns the PIN. The board uses Nefrita's PIN server unless you give it another one. This guide runs your own, the PIN server of the [Nefrita firmware](https://github.com/Sharmaz/Jade), in Docker and behind TLS, and loads it on a board from the app.

> Nefrita is in early development: nothing here is ready to hold funds.

The board and the PIN server encrypt and authenticate what they send each other with the key pair of the PIN server, whose public key you confirm on the board. TLS adds a layer on top and hides which request the app forwards. For TLS, the PIN server has a server certificate signed by a certificate authority of your own. The board stores the certificate of that certificate authority, so the server certificate can be renewed without touching the board.

The commands are for bash or zsh, on macOS or Linux, and they use `localhost`, to try the PIN server on the computer that runs the app; "A PIN server on another computer" says what changes for a real PIN server. Run the blocks in order, in the same shell.

## What you need

- Docker and git.
- The Nefrita app, and a board without a saved wallet: a board that stores a wallet with a PIN refuses a new PIN server. "Factory Reset", in the menu that opens when you press a button on the first screen of the board as it powers on, erases everything the board stores, the wallet included.

The image of the PIN server is built for `linux/amd64`, as in the guides of the firmware and the app. On an ARM computer, such as a Mac with Apple silicon or a Raspberry Pi, Docker runs it under emulation.

## Prepare the shell

```bash
pin_server_host=localhost
pin_server_port=8443
pin_server_directory="$HOME/nefrita-pin-server"
firmware_commit=e18deffd96fb293730a51c9874c73ab4ca6e841a
debian_image=debian:trixie@sha256:4ae67669760b807c19f23902a3fd7c121a6a70cf2ae709035674b23e712e4d62
pin_server_url="https://${pin_server_host}:${pin_server_port}"
firmware_directory="$pin_server_directory/firmware"
pin_server_key_pair_directory="$pin_server_directory/pin-server-key-pair"
certificate_authority_directory="$pin_server_directory/certificate-authority"
server_certificate_directory="$pin_server_directory/server-certificate"
nginx_tls_configuration_file="$pin_server_directory/nginx-tls.conf"
file_owner="$(id -u):$(id -g)"
wait_for_pin_server() {
  curl --silent --fail --output /dev/null --retry 60 --retry-delay 1 --retry-all-errors \
    --cacert "$certificate_authority_directory/certificate.pem" "$pin_server_url/"
}
```

`pin_server_host` is the name or the IPv4 address with which the app reaches the PIN server, and `pin_server_port` is the port where it listens with TLS. `pin_server_directory` keeps the firmware, the keys and the certificates. `firmware_commit` is the firmware commit that the continuous integration of the app tests, and `debian_image` is the Debian image that the PIN server of that commit is built on, which also creates the certificates. `wait_for_pin_server` waits until the PIN server answers through TLS, trusting only your certificate authority. Run this block again in every new shell.

## Build the PIN server image

The PIN server is the `pinserver` submodule of the firmware, which comes from Blockstream's `blind_pin_server` repository. Create the directory, which only you can open because it keeps the keys, then clone the firmware, check out the tested commit, initialize that submodule alone and build the image:

```bash
mkdir -p "$pin_server_directory"
chmod 700 "$pin_server_directory"
[ -d "$firmware_directory" ] || git clone https://github.com/Sharmaz/Jade.git "$firmware_directory"
git -C "$firmware_directory" fetch origin
git -C "$firmware_directory" checkout "$firmware_commit"
git -C "$firmware_directory" submodule update --init pinserver
docker build --platform linux/amd64 --tag pin-server "$firmware_directory/pinserver"
```

## Create the key pair of the PIN server

The key pair is the identity of your PIN server: the board stores its public key, and the wallets registered with the PIN server unlock only with its private key. This block writes `server_private_key.key` and `server_public_key.pub` in `pin-server-key-pair/`, and keeps a key pair that already exists:

```bash
mkdir -p "$pin_server_key_pair_directory"
docker run --rm --platform linux/amd64 --user "$file_owner" \
  --volume "$pin_server_key_pair_directory":/pin-server-key-pair \
  --workdir /pin-server-key-pair --env PYTHONPATH=/ \
  pin-server python3 -m pinserver.generateserverkey
```

## Create the certificate authority

The board stores the certificate of this certificate authority, which lasts 20 years. This block creates a P-256 key and its certificate with the openssl of the Debian image, and keeps a certificate authority that already exists, because a new one would no longer match the certificate stored on the board:

```bash
mkdir -p "$certificate_authority_directory"
[ -e "$certificate_authority_directory/certificate.pem" ] || docker run --rm \
  --volume "$certificate_authority_directory":/certificate-authority \
  --workdir /certificate-authority --env FILE_OWNER="$file_owner" \
  --env DEBIAN_FRONTEND=noninteractive "$debian_image" bash -c '
  set -e
  apt-get update -qq
  apt-get install -qq --yes --no-install-recommends openssl >/dev/null
  openssl req -x509 -newkey ec -pkeyopt ec_paramgen_curve:P-256 -noenc \
    -keyout key.pem -out certificate.pem -days 7300 \
    -subj "/CN=Nefrita PIN server certificate authority"
  chmod 600 key.pem
  chown "$FILE_OWNER" key.pem certificate.pem'
```

## Create the server certificate

The server certificate is created for `pin_server_host`, as a name or as an IPv4 address, signed by the certificate authority, and lasts 825 days:

```bash
mkdir -p "$server_certificate_directory"
docker run --rm --volume "$certificate_authority_directory":/certificate-authority:ro \
  --volume "$server_certificate_directory":/server-certificate \
  --workdir /server-certificate --env PIN_SERVER_HOST="$pin_server_host" \
  --env FILE_OWNER="$file_owner" --env DEBIAN_FRONTEND=noninteractive \
  "$debian_image" bash -c '
  set -e
  apt-get update -qq
  apt-get install -qq --yes --no-install-recommends openssl >/dev/null
  case "$PIN_SERVER_HOST" in
    *[!0-9.]*) subject_alternative_name="DNS:$PIN_SERVER_HOST" ;;
    *) subject_alternative_name="IP:$PIN_SERVER_HOST" ;;
  esac
  openssl req -x509 -newkey ec -pkeyopt ec_paramgen_curve:P-256 -noenc \
    -keyout key.pem -out certificate.pem -days 825 -subj "/CN=$PIN_SERVER_HOST" \
    -addext "subjectAltName=$subject_alternative_name" \
    -addext "basicConstraints=critical,CA:FALSE" \
    -CA /certificate-authority/certificate.pem -CAkey /certificate-authority/key.pem
  chmod 600 key.pem
  chown "$FILE_OWNER" key.pem certificate.pem'
```

## Write the nginx TLS configuration

The image serves the PIN server through nginx, without TLS. This block writes the same configuration with TLS, which the PIN server mounts in place of its own:

```bash
printf '%s\n' \
  'server {' \
  '    listen 8096 ssl;' \
  '    ssl_certificate /server-certificate/certificate.pem;' \
  '    ssl_certificate_key /server-certificate/key.pem;' \
  '    root /usr/share/nginx/html;' \
  '    add_header Access-Control-Allow-Origin * always;' \
  "    add_header Access-Control-Allow-Methods 'GET, POST' always;" \
  "    add_header Access-Control-Expose-Headers 'x-total-results';" \
  '    location / {' \
  '        try_files $uri @wsgi;' \
  '    }' \
  '    location @wsgi {' \
  '        include uwsgi_params;' \
  '        uwsgi_pass unix:/tmp/uwsgi.socket;' \
  '    }' \
  '    location = /healthz {' \
  '        return 200;' \
  '    }' \
  '}' \
  > "$nginx_tls_configuration_file"
```

## Start the PIN server

The PIN server listens with TLS on `pin_server_port`, keeps the PINs in the Docker volume `nefrita-pin-server-pins` and starts again whenever Docker starts. The key of the certificate authority never enters it:

```bash
docker run --detach --name nefrita-pin-server --platform linux/amd64 \
  --restart unless-stopped --publish "$pin_server_port":8096 \
  --volume nefrita-pin-server-pins:/pins \
  --volume "$pin_server_key_pair_directory/server_private_key.key":/server_private_key.key:ro \
  --volume "$server_certificate_directory":/server-certificate:ro \
  --volume "$nginx_tls_configuration_file":/etc/nginx/conf.d/default.conf:ro \
  pin-server
wait_for_pin_server
```

To start the PIN server from scratch, remove the old container first with `docker rm --force nefrita-pin-server`; the PINs stay in their volume.

## Load the PIN server in the app

This block prints what to enter in the app, and what to compare on the board:

```bash
echo "URL: $pin_server_url"
docker run --rm \
  --volume "$pin_server_key_pair_directory/server_public_key.pub":/server_public_key.pub:ro \
  --volume "$certificate_authority_directory/certificate.pem":/certificate-authority.pem:ro \
  "$debian_image" bash -c '
  hexadecimal_public_key=$(od -An -tx1 -v /server_public_key.pub | tr -d " \n")
  certificate_hash=$(sha256sum /certificate-authority.pem | cut -c1-64)
  echo "Public key: $hexadecimal_public_key"
  echo "Certificate:"
  cat /certificate-authority.pem
  echo "Public key on the board: $(echo "$hexadecimal_public_key" | fold -w 8 | paste -sd " ")"
  echo "Certificate hash on the board: $(echo "$certificate_hash" | fold -w 8 | paste -sd " ")"'
```

In the app, connect the board and choose "Use my own PIN server". Enter the URL and the public key, leave the onion URL empty, and paste the certificate, from `-----BEGIN CERTIFICATE-----` to `-----END CERTIFICATE-----`: it is the certificate of your certificate authority, not the server certificate. The app checks the details and that the PIN server answers, and then the board shows them. Check that the URL and the public key on the board, in groups of 8 characters, are the ones above, and that the hash of the certificate on the next screen matches the one printed as "Certificate hash on the board". Confirm on the board only if everything matches. Then "Create the wallet", or "Save the wallet", registers the PIN with your PIN server, and once the wallet is saved the app shows its URL.

## Renew the server certificate

Before the server certificate ends, or if its key leaks, run the block of "Create the server certificate" again, and then restart the PIN server so that nginx loads the new server certificate. The board needs nothing, because it trusts your certificate authority and not the server certificate:

```bash
docker restart nefrita-pin-server
wait_for_pin_server
```

## Back up the PIN server

Keep a copy of these, away from the PIN server:

- `pin-server-key-pair/server_private_key.key` and the PINs: without them, the wallets registered with the PIN server no longer unlock, and only their recovery phrases bring them back.
- `certificate-authority/`: without its key, a new server certificate would need a new certificate authority, which does not match the certificate stored on the board, and the app could no longer reach the PIN server.

This block copies the PINs from their volume to `pins-backup.tar.gz`, in `pin_server_directory`:

```bash
docker run --rm --volume nefrita-pin-server-pins:/pins:ro \
  --volume "$pin_server_directory":/backup --env FILE_OWNER="$file_owner" \
  "$debian_image" bash -c '
  set -e
  tar --create --gzip --file /backup/pins-backup.tar.gz --directory /pins .
  chown "$FILE_OWNER" /backup/pins-backup.tar.gz'
```

## Stop and start the PIN server

`docker stop` stops the PIN server, and it stays stopped, even when Docker starts again, until `docker start`:

```bash
docker stop nefrita-pin-server
docker start nefrita-pin-server
wait_for_pin_server
```

## A PIN server on another computer

To run the PIN server on another computer, follow this guide there, with Docker and git, and change the first three variables, for example:

```text
pin_server_host=pin-server.home.arpa
pin_server_port=8443
pin_server_directory="$HOME/nefrita-pin-server"
```

`pin_server_host` must be the name or the IPv4 address with which the computer of the app reaches the PIN server, because the server certificate is created for it: if it changes, create the server certificate again. The port must be open on that computer. Then enter in the app, on your computer, the values that "Load the PIN server in the app" prints. The app does not reach a PIN server through Tor yet.
