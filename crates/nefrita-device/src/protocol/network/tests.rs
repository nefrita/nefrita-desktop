use super::*;

#[test]
fn networks_encode_as_their_protocol_names() {
    let networks_and_protocol_names = [
        (Network::Xbt, "xbt"),
        (Network::XbtTestnet4, "xbt-testnet4"),
        (Network::XbtRegtest, "xbt-regtest"),
    ];
    for (network, protocol_name) in networks_and_protocol_names {
        let encoded_network = minicbor_serde::to_vec(network).expect("the network should encode");
        let decoded_protocol_name: String =
            minicbor_serde::from_slice(&encoded_network).expect("the protocol name should decode");
        assert_eq!(decoded_protocol_name, protocol_name, "network {network:?}");
    }
}
