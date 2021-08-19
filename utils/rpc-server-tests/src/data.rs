pub(crate) const SCHEMA: &str = r#"
{
    "$schema":"http://json-schema.org/draft-04/schema#",
    "type":"object",
    "properties":{
        "jsonrpc":{
            "type":"string"
        },
        "id":{
            "type":"integer"
        },
        "result":{
            "type":"object",
            "properties":{
                "api_version":{
                    "type":"string"
                },
                "chainspec_name":{
                    "type":"string"
                },
                "starting_state_root_hash":{
                    "type":"string"
                },
                "peers":{
                    "type":"array",
                    "items":[
                        {
                            "type":"object",
                            "properties":{
                                "node_id":{
                                    "type":"string"
                                },
                                "address":{
                                    "type":"string"
                                }
                            },
                            "required":[
                                "node_id",
                                "address"
                            ]
                        },
                        {
                            "type":"object",
                            "properties":{
                                "node_id":{
                                    "type":"string"
                                },
                                "address":{
                                    "type":"string"
                                }
                            },
                            "required":[
                                "node_id",
                                "address"
                            ]
                        },
                        {
                            "type":"object",
                            "properties":{
                                "node_id":{
                                    "type":"string"
                                },
                                "address":{
                                    "type":"string"
                                }
                            },
                            "required":[
                                "node_id",
                                "address"
                            ]
                        },
                        {
                            "type":"object",
                            "properties":{
                                "node_id":{
                                    "type":"string"
                                },
                                "address":{
                                    "type":"string"
                                }
                            },
                            "required":[
                                "node_id",
                                "address"
                            ]
                        }
                    ]
                },
                "last_added_block_info":{
                    "type":"object",
                    "properties":{
                        "hash":{
                            "type":"string"
                        },
                        "timestamp":{
                            "type":"string"
                        },
                        "era_id":{
                            "type":"integer"
                        },
                        "height":{
                            "type":"integer"
                        },
                        "state_root_hash":{
                            "type":"string"
                        },
                        "creator":{
                            "type":"string"
                        }
                    },
                    "required":[
                        "hash",
                        "timestamp",
                        "era_id",
                        "height",
                        "state_root_hash",
                        "creator"
                    ]
                },
                "our_public_signing_key":{
                    "type":"string"
                },
                "round_length":{
                    "type":"string"
                },
                "next_upgrade":{
                    "type":"null"
                },
                "build_version":{
                    "type":"string"
                },
                "uptime":{
                    "type":"string"
                }
            },
            "required":[
                "api_version",
                "chainspec_name",
                "starting_state_root_hash",
                "peers",
                "last_added_block_info",
                "our_public_signing_key",
                "round_length",
                "next_upgrade",
                "build_version",
                "uptime"
            ]
        }
    },
    "required":[
        "jsonrpc",
        "id",
        "result"
    ]
}

"#;
