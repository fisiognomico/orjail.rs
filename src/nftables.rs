use nftables::{batch::Batch, expr, schema, stmt, types};

const PREROUTING_CHAIN: &str = "PREROUTING";
const DNAT_PRIORITY: i32 = -100;
const NAT_TABLE: &str = "nat";
const TOR_DNS: u32 = 5353;
const IFNAME: &str = "test";
const PROXY_PORT: u32 = 9050;


/// Create nftables chain with acccept policy
/// Does not set a default hook or priority
fn create_chain(name: &str, chain_type: types::NfChainType, hook: types::NfHook, priority: i32) -> schema::NfListObject {
    schema::NfListObject::Chain(schema::Chain::new(
        types::NfFamily::IP,
        NAT_TABLE.to_string(),
        name.to_string(),
        Some(chain_type),
        Some(hook),
        Some(priority),
        None,
        Some(types::NfChainPolicy::Accept)
    ))
}

/// Consider that we will add rules only to the NAT table, we do not need anything else
fn create_rule(chain: &str, conditions: Vec<stmt::Statement>) -> schema::NfListObject {
    schema::NfListObject::Rule(schema::Rule::new(
        types::NfFamily::IP,
        NAT_TABLE.to_string(),
        chain.to_string(),
        conditions
    ))
}

fn tproxy(ifname: &str, proxy_port: u32, proto: &str) -> schema::NfListObject {
    create_rule(
        PREROUTING_CHAIN,
        vec![
            // Condition: interface_name = ifname
            stmt::Statement::Match(stmt::Match {
                left: expr::Expression::Named(expr::NamedExpression::Meta(
                    expr::Meta { 
                        key: expr::MetaKey::Iifname,
                })),
                right: expr::Expression::String(ifname.to_string()),
                op: stmt::Operator::EQ
            }),
            stmt::Statement::Match(stmt::Match {
                left: expr::Expression::Named(expr::NamedExpression::Payload(
                    expr::Payload::PayloadField(expr::PayloadField {
                        protocol: "ip".to_string(),
                        field: "protocol".to_string()
                    }),
                )),
                right: expr::Expression::String(proto.to_string()),
                op: stmt::Operator::EQ,
            }),
            // Then REDIRECT everything to proxy_port
            stmt::Statement::Redirect(Some(stmt::NAT {
                addr: None,
                family: None,
                port: Some(proxy_port),
                flags: None
            }))
        ],
    )
}

fn dns_dnat(dns_port: u32, proto: &str) -> schema::NfListObject {
    create_rule(
        PREROUTING_CHAIN,
        vec![
            // Condition: dport == 53
            stmt::Statement::Match(stmt::Match {
                left: expr::Expression::Named(expr::NamedExpression::Payload(
                    expr::Payload::PayloadField(expr::PayloadField {
                        protocol: proto.to_string(),
                        field: "dport".to_string(),
                    }),
                )),
                right: expr::Expression::Number(53),
                op: stmt::Operator::EQ }),
            // Then REDIRECT to dns_port
            stmt::Statement::Redirect(Some(stmt::NAT {
                addr: None,
                // TODO support IPv6
                family: Some(stmt::NATFamily::IP),
                port: Some(dns_port),
                flags: None
            })),
        ],
    )
}

/// Applies a ruleset to nftables.
fn example_ruleset() -> schema::Nftables {
    let mut batch = Batch::new();
    batch.add(schema::NfListObject::Table(schema::Table::new(
        types::NfFamily::IP,
        NAT_TABLE.to_string(),
    )));

    batch.add(
        create_chain(PREROUTING_CHAIN, types::NfChainType::NAT, types::NfHook::Prerouting, DNAT_PRIORITY)
    );

    batch.add(dns_dnat(TOR_DNS, "udp"));
    batch.add(dns_dnat(TOR_DNS, "tcp"));
    batch.add(tproxy(IFNAME, PROXY_PORT, "udp"));
    batch.add(tproxy(IFNAME, PROXY_PORT, "tcp"));
    // Chain delivery
    batch.to_nftables()
}

pub fn test_apply_ruleset() {
    let ruleset = example_ruleset();
    nftables::helper::apply_ruleset(&ruleset, None, None).unwrap();
}

