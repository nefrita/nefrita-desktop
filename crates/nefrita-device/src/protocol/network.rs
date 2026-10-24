use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Network {
    #[serde(rename = "xbt")]
    Xbt,
    #[serde(rename = "xbt-testnet4")]
    XbtTestnet4,
    #[serde(rename = "xbt-regtest")]
    XbtRegtest,
}

#[cfg(test)]
mod tests;
