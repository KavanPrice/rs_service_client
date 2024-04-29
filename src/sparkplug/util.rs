const SP_PREFIX: &str = "spBv1.0";

#[derive(Clone, PartialEq)]
pub struct Address {
    group: String,
    node: String,
    address_type: AddressType,
}

impl Address {
    pub fn parse(str: &str) -> Option<Address> {
        let config_vec: Vec<&str> = str.split('/').collect();

        return match (config_vec.get(0), config_vec.get(1), config_vec.get(2)) {
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
        };
    }

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

    pub fn to_string(&self) -> String {
        let device_path = match &self.address_type {
            AddressType::Device(device_name) => Some(format!("/{}", device_name)),
            AddressType::Node => None,
        };
        format!(
            "{}/{}{}",
            self.group,
            self.node,
            device_path.unwrap_or_default()
        )
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

    pub fn topic(&self, topic_type: TopicType) -> Topic {
        Topic {
            prefix: String::from(SP_PREFIX),
            address: self.clone(),
            topic_type,
        }
    }

    pub fn parent_node(&self) -> Address {
        Address {
            group: self.group.clone(),
            node: self.node.clone(),
            address_type: AddressType::Node,
        }
    }

    pub fn child_device(&self, device_str: &str) -> Option<Address> {
        return match self.address_type {
            AddressType::Node => Some(Address {
                group: self.group.clone(),
                node: self.node.clone(),
                address_type: AddressType::Device(String::from(device_str)),
            }),
            AddressType::Device(_) => None,
        };
    }

    pub fn is_child_of(&self, parent: Address) -> bool {
        self.parent_node() == parent
    }
}

#[derive(Clone, PartialEq)]
pub enum AddressType {
    // Wraps the device name if the address is for a device
    Device(String),
    Node,
}

impl AddressType {
    fn to_string(&self) -> String {
        String::from(match self {
            AddressType::Device(_) => "D",
            AddressType::Node => "N",
        })
    }

    pub fn from_type_str(type_str: &str, device_str: Option<&str>) -> Option<AddressType> {
        match (type_str, device_str) {
            ("N", _) => Some(AddressType::Node),
            ("D", Some(str)) => Some(AddressType::Device(String::from(str))),
            _ => None,
        }
    }
}

pub struct Topic {
    prefix: String,
    address: Address,
    topic_type: TopicType,
}

impl Topic {
    pub fn parse(str: &str) -> Option<Topic> {
        let config_vec: Vec<&str> = str.split('/').collect();

        if config_vec.len() != 4 || config_vec.len() != 5 {
            return None;
        }

        if let Some(pref) = config_vec.get(0) {
            if *pref != SP_PREFIX {
                return None;
            }
        } else {
            return None;
        }

        return if let Some(addr) = match (config_vec.get(1), config_vec.get(3), config_vec.get(4)) {
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
                if let Some(topic_type) = TopicType::from_type_str(type_str) {
                    Some(Topic {
                        prefix: String::from(SP_PREFIX),
                        address: addr,
                        topic_type,
                    })
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };
    }

    pub fn to_string(&self) -> String {
        let device_path = match &self.address.address_type {
            AddressType::Device(device_name) => Some(format!("/{}", device_name)),
            AddressType::Node => None,
        };
        format!(
            "{}/{}/{}/{}{}",
            self.prefix,
            self.address.group,
            self.topic_type.to_string(),
            self.address.node,
            device_path.unwrap_or_default()
        )
    }
}

pub enum TopicType {
    Any,
    BIRTH,
    CMD,
    DATA,
    DEATH,
}

impl TopicType {
    pub fn to_string(&self) -> String {
        String::from(match &self {
            TopicType::Any => "+",
            TopicType::BIRTH => "BIRTH",
            TopicType::CMD => "CMD",
            TopicType::DATA => "DATA",
            TopicType::DEATH => "DEATH",
        })
    }

    pub fn from_type_str(type_str: &str) -> Option<TopicType> {
        match type_str {
            "+" => Some(TopicType::Any),
            "BIRTH" => Some(TopicType::BIRTH),
            "CMD" => Some(TopicType::CMD),
            "DATA" => Some(TopicType::DATA),
            "DEATH" => Some(TopicType::DEATH),
            _ => None,
        }
    }
}
