pub mod types {
    pub const APP: uuid::Uuid = uuid::uuid!("d319bd87-f42b-4b66-be4f-f82ff48b93f0");
    pub const CLASS: uuid::Uuid = uuid::uuid!("04a1c90d-2295-4cbe-b33a-74eded62cbf1");
    pub const DEVICE: uuid::Uuid = uuid::uuid!("18773d6d-a70d-443a-b29a-3f1583195290");
    pub const EDGE_AGENT: uuid::Uuid = uuid::uuid!("00da3c0b-f62b-4761-a689-39ad0c33f864");
    pub const GIT_REPO: uuid::Uuid = uuid::uuid!("d25f2afc-1ab8-4d27-b51b-d02314624e3e");
    pub const GIT_REPO_GROUP: uuid::Uuid = uuid::uuid!("b03d4dfe-7e78-4252-8e62-af594cf316c9");
    pub const PERM_GROUP: uuid::Uuid = uuid::uuid!("ac0d5288-6136-4ced-a372-325fbbcdd70d");
    pub const PERMISSION: uuid::Uuid = uuid::uuid!("8ae784bb-c4b5-4995-9bf6-799b3c7f21ad");
    pub const REQUIREMENT: uuid::Uuid = uuid::uuid!("b419cbc2-ab0f-4311-bd9e-f0591f7e88cb");
    pub const SCHEMA: uuid::Uuid = uuid::uuid!("83ee28d4-023e-4c2c-ab86-12c24e86372c");
    pub const SERVICE: uuid::Uuid = uuid::uuid!("265d481f-87a7-4f93-8fc6-53fa64dc11bb");
    pub const SPECIAL: uuid::Uuid = uuid::uuid!("ddb132e4-5cdd-49c8-b9b1-2f35879eab6d");
}

pub mod special {
    pub const NULL: uuid::Uuid = uuid::uuid!("00000000-0000-0000-0000-000000000000");
    pub const FACTORY_PLUS: uuid::Uuid = uuid::uuid!("11ad7b32-1d32-4c4a-b0c9-fa049208939a");
    pub const SELF: uuid::Uuid = uuid::uuid!("5855a1cc-46d8-4b16-84f8-ab3916ecb230");
}

pub mod app {
    pub const REGISTRATION: uuid::Uuid = uuid::uuid!("cb40bed5-49ad-4443-a7f5-08c75009da8f");
    pub const INFO: uuid::Uuid = uuid::uuid!("64a8bfa9-7772-45c4-9d1a-9e6290690957");
    pub const SPARKPLUG_ADDRESS: uuid::Uuid = uuid::uuid!("8e32801b-f35a-4cbf-a5c3-2af64d3debd7");
    pub const CONFIG_SCHEMA: uuid::Uuid = uuid::uuid!("dbd8a535-52ba-4f6e-b4f8-9b71aefe09d3");
    pub const SERVICE_CONFIG: uuid::Uuid = uuid::uuid!("5b47881c-b012-4040-945c-eacafca539b2");
}

pub mod schema {
    pub const DEVICE_INFORMATION: uuid::Uuid = uuid::uuid!("2dd093e9-1450-44c5-be8c-c0d78e48219b");
    pub const SERVICE: uuid::Uuid = uuid::uuid!("05688a03-730e-4cda-9932-172e2c62e45c");
}

pub mod service {
    pub const DIRECTORY: uuid::Uuid = uuid::uuid!("af4a1d66-e6f7-43c4-8a67-0fa3be2b1cf9");
    pub const CONFIG_DB: uuid::Uuid = uuid::uuid!("af15f175-78a0-4e05-97c0-2a0bb82b9f3b");
    pub const AUTHENTICATION: uuid::Uuid = uuid::uuid!("cab2642a-f7d9-42e5-8845-8f35affe1fd4");
    pub const COMMAND_ESCALATION: uuid::Uuid = uuid::uuid!("78ea7071-24ac-4916-8351-aa3e549d8ccd");
    pub const MQTT: uuid::Uuid = uuid::uuid!("feb27ba3-bd2c-4916-9269-79a61ebc4a47");
    pub const GIT: uuid::Uuid = uuid::uuid!("7adf4db0-2e7b-4a68-ab9d-376f4c5ce14b");
    pub const CLUSTERS: uuid::Uuid = uuid::uuid!("2706aa43-a826-441e-9cec-cd3d4ce623c2");
}

pub mod permission {
    pub mod auth {
        pub const READ_ACL: uuid::Uuid = uuid::uuid!("ba566181-0e8a-405b-b16e-3fb89130fbee");
        pub const MANAGE_KERBEROS: uuid::Uuid = uuid::uuid!("327c4cc8-9c46-4e1e-bb6b-257ace37b0f6");
        pub const MANAGE_ACL: uuid::Uuid = uuid::uuid!("3a41f5ce-fc08-4669-9762-ec9e71061168");
        pub const MANAGE_GROUP: uuid::Uuid = uuid::uuid!("be9b6d47-c845-49b2-b9d5-d87b83f11c3b");
    }
    pub mod cmd_esc {
        pub const REBIRTH: uuid::Uuid = uuid::uuid!("fbb9c25d-386d-4966-a325-f16471d9f7be");
    }
    pub mod config_db {
        pub const READ_CONFIG: uuid::Uuid = uuid::uuid!("4a339562-cd57-408d-9d1a-6529a383ea4b");
        pub const WRITE_CONFIG: uuid::Uuid = uuid::uuid!("6c799ccb-d2ad-4715-a2a7-3c8728d6c0bf");
        pub const MANAGE_OBJECTS: uuid::Uuid = uuid::uuid!("f0b7917b-d475-4888-9d5a-2af96b3c26b6");
    }
    pub mod directory {
        pub const ADVERTISE_SERVICE: uuid::Uuid = uuid::uuid!("4db4c39a-f18d-4e83-aeb0-5af2c14ddc2b");
    }
}
