use serde::{Serialize, Serializer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Network {
    Xbt,
    XbtTestnet4,
    XbtRegtest,
}

impl Network {
    pub const ALL: [Self; 3] = [Self::Xbt, Self::XbtTestnet4, Self::XbtRegtest];

    pub fn protocol_name(self) -> &'static str {
        match self {
            Self::Xbt => "xbt",
            Self::XbtTestnet4 => "xbt-testnet4",
            Self::XbtRegtest => "xbt-regtest",
        }
    }

    pub fn from_protocol_name(protocol_name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|network| network.protocol_name() == protocol_name)
    }
}

impl Serialize for Network {
    fn serialize<NetworkSerializer: Serializer>(
        &self,
        serializer: NetworkSerializer,
    ) -> Result<NetworkSerializer::Ok, NetworkSerializer::Error> {
        serializer.serialize_str(self.protocol_name())
    }
}

#[cfg(test)]
mod tests;
