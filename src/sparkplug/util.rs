const SP_PREFIX: &str = "spBv1.0";

pub mod address {
    //! This module contains structs and implementations for handling Sparkplug addresses.

    use std::fmt::{Display, Formatter};
    use std::str::FromStr;

    use crate::error::SparkplugError;
    use crate::sparkplug::util::topic::{Topic, TopicType};
    use crate::sparkplug::util::SP_PREFIX;

    #[derive(Clone, PartialEq)]
    pub struct Address {
        pub group: String,
        pub node: String,
        pub address_type: AddressType,
    }

    impl Address {
        pub fn matches(&self, other: &Address) -> bool {
            fn wild(p: &String, a: &String) -> bool {
                p == a || p == "+"
            }

            fn wild_address_type(p: &AddressType, a: &AddressType) -> bool {
                p == a || (*p == AddressType::Device("+".to_owned()) && *a != AddressType::Node)
            }

            wild(&self.group, &other.group)
                && wild(&self.node, &other.node)
                && wild_address_type(&self.address_type, &other.address_type)
        }

        pub fn is_device(&self) -> bool {
            match self.address_type {
                AddressType::Device(_) => true,
                AddressType::Node => false,
            }
        }

        pub fn topic_kind(&self) -> String {
            self.address_type.to_string()
        }

        pub fn to_topic(&self, topic_type: TopicType) -> Topic {
            Topic {
                prefix: String::from(SP_PREFIX),
                address: self.clone(),
                topic_type,
            }
        }
    }

    impl FromStr for Address {
        type Err = SparkplugError;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            let config_vec: Vec<&str> = s.split('/').collect();

            return match (config_vec.get(0), config_vec.get(1), config_vec.get(2)) {
                (Some(&group_str), Some(&node_str), Some(&device_str)) => Ok(Address {
                    group: String::from(group_str),
                    node: String::from(node_str),
                    address_type: AddressType::Device(String::from(device_str)),
                }),
                (Some(&group_str), Some(&node_str), None) => Ok(Address {
                    group: String::from(group_str),
                    node: String::from(node_str),
                    address_type: AddressType::Node,
                }),
                _ => Err(SparkplugError {
                    message: String::from("Couldn't parse address."),
                }),
            };
        }
    }

    impl Display for Address {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            let device_path = match &self.address_type {
                AddressType::Device(device_name) => Some(format!("/{}", device_name)),
                AddressType::Node => None,
            };
            write!(
                f,
                "{}/{}{}",
                self.group,
                self.node,
                device_path.unwrap_or_default()
            )
        }
    }

    #[derive(Clone, PartialEq)]
    pub enum AddressType {
        // Wraps the device name if the address is for a device
        Device(String),
        Node,
    }

    impl Display for AddressType {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "{}",
                match self {
                    AddressType::Device(_) => "D",
                    AddressType::Node => "N",
                }
            )
        }
    }
}

pub mod topic {
    //! This module contains structs and implementations for handling Sparkplug topics.

    use std::fmt::{Display, Formatter};
    use std::str::FromStr;

    use crate::error::SparkplugError;
    use crate::sparkplug::util::address::{Address, AddressType};
    use crate::sparkplug::util::SP_PREFIX;

    pub struct Topic {
        pub prefix: String,
        pub address: Address,
        pub topic_type: TopicType,
    }

    impl FromStr for Topic {
        type Err = SparkplugError;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            let config_vec: Vec<&str> = s.split('/').collect();

            if config_vec.len() != 4 && config_vec.len() != 5 {
                return Err(SparkplugError {
                    message: String::from("Incorrect topic length"),
                });
            }

            if let Some(pref) = config_vec.first() {
                if *pref != SP_PREFIX {
                    return Err(SparkplugError {
                        message: String::from("Incorrect Sparkplug prefix"),
                    });
                }
            } else {
                return Err(SparkplugError {
                    message: String::from("No Sparkplug prefix"),
                });
            }

            if let Some(addr) = match (config_vec.get(1), config_vec.get(3), config_vec.get(4)) {
                (Some(&group_str), Some(&node_str), Some(&device_str)) => Some(Address {
                    group: String::from(group_str),
                    node: String::from(node_str),
                    address_type: AddressType::Device(String::from(device_str)),
                }),
                (Some(&group_str), Some(&node_str), None) => Some(Address {
                    group: String::from(group_str),
                    node: String::from(node_str),
                    address_type: AddressType::Node,
                }),
                _ => None,
            } {
                if let Some(type_str) = config_vec.get(2) {
                    let topic_type = TopicType::from_str(type_str)?;
                    Ok(Topic {
                        prefix: String::from(SP_PREFIX),
                        address: addr,
                        topic_type,
                    })
                } else {
                    Err(SparkplugError {
                        message: String::from("No topic type"),
                    })
                }
            } else {
                Err(SparkplugError {
                    message: String::from("Couldn't get topic components"),
                })
            }
        }
    }

    impl Display for Topic {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            let device_path = match &self.address.address_type {
                AddressType::Device(device_name) => Some(format!("/{}", device_name)),
                AddressType::Node => None,
            };
            write!(
                f,
                "{}/{}/{}/{}{}",
                self.prefix,
                self.address.group,
                self.topic_type,
                self.address.node,
                device_path.unwrap_or_default()
            )
        }
    }

    pub enum TopicType {
        Any,
        NBIRTH,
        NCMD,
        NDATA,
        NDEATH,
        DBIRTH,
        DCMD,
        DDATA,
        DDEATH,
    }

    impl FromStr for TopicType {
        type Err = SparkplugError;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            match s {
                "+" => Ok(TopicType::Any),
                "NBIRTH" => Ok(TopicType::NBIRTH),
                "NCMD" => Ok(TopicType::NCMD),
                "NDATA" => Ok(TopicType::NDATA),
                "NDEATH" => Ok(TopicType::NDEATH),
                "DBIRTH" => Ok(TopicType::DBIRTH),
                "DCMD" => Ok(TopicType::DCMD),
                "DDATA" => Ok(TopicType::DDATA),
                "DDEATH" => Ok(TopicType::DDEATH),
                _ => Err(SparkplugError {
                    message: String::from("Couldn't determine topic type"),
                }),
            }
        }
    }

    impl Display for TopicType {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "{}",
                match &self {
                    TopicType::Any => "+",
                    TopicType::NBIRTH => "NBIRTH",
                    TopicType::NCMD => "NCMD",
                    TopicType::NDATA => "NDATA",
                    TopicType::NDEATH => "NDEATH",
                    TopicType::DBIRTH => "DBIRTH",
                    TopicType::DCMD => "DCMD",
                    TopicType::DDATA => "DDATA",
                    TopicType::DDEATH => "DDEATH",
                }
            )
        }
    }
}
