#![allow(unused_must_use)]
//! Registry sandbox tests: agents and robots register on NEAR, pay their own storage, get refunds.
use factor_integration::*;
use near_api::{AccountId, NearGas, NearToken, NetworkConfig, Signer};
use serde_json::{Value, json};
use std::sync::Arc;

struct R {
    _sb: near_sandbox::Sandbox,
    net: NetworkConfig,
    signer: Arc<Signer>,
    registry: AccountId,
}

async fn setup(accounts: &[&str]) -> TestResult<R> {
    let sb = near_sandbox::Sandbox::start_sandbox().await?;
    let net = NetworkConfig::from_rpc_url("sandbox", sb.rpc_addr.parse()?);
    let signer = Signer::from_secret_key(near_sandbox::config::DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY.parse()?)?;
    let registry = id("registry.sandbox");
    for a in std::iter::once("registry").chain(accounts.iter().copied()) {
        sb.create_account(id(&format!("{a}.sandbox")).as_str().parse()?).initial_balance(NearToken::from_near(20)).send().await?;
    }
    near_api::Contract::deploy(registry.clone())
        .use_code(wasm("factor_registry"))
        .with_init_call("new", json!({}))?
        .with_signer(signer.clone())
        .send_to(&net)
        .await?
        .assert_success();
    Ok(R { _sb: sb, net, signer, registry })
}

impl R {
    async fn call(&self, who: &str, method: &str, args: Value, deposit: NearToken) -> TestResult<near_api::types::transaction::result::ExecutionFinalResult> {
        Ok(near_api::Contract(self.registry.clone())
            .call_function(method, args)
            .transaction()
            .deposit(deposit)
            .gas(NearGas::from_tgas(50))
            .with_signer(id(&format!("{who}.sandbox")), self.signer.clone())
            .send_to(&self.net)
            .await?
            .into_full()
            .unwrap())
    }
    async fn view<T: serde::de::DeserializeOwned + Send + Sync>(&self, method: &str, args: Value) -> TestResult<T> {
        Ok(near_api::Contract(self.registry.clone()).call_function(method, args).read_only().fetch_from(&self.net).await?.data)
    }
    async fn bal(&self, who: &str) -> u128 {
        near_api::Account(id(&format!("{who}.sandbox"))).tokens().near_balance().fetch_from(&self.net).await.unwrap().total.as_yoctonear()
    }
}

fn input(name: &str, kind: &str) -> Value {
    json!({"input": {"name": name, "kind": kind, "description": "buys invoices from healthy DAOs",
        "capabilities": ["buy-invoices", "risk-scoring"], "endpoint": "https://agent.example/mcp",
        "strategy": {"min_score": 70, "min_apr_pct": 40, "max_price": "10000000000000000000000000"}}})
}

#[tokio::test]
async fn t_agent_and_robot_register_and_list() -> TestResult<()> {
    let r = setup(&["carol", "rover"]).await?;
    let before = r.bal("carol").await;
    assert_ok(r.call("carol", "register", input("Carol Capital", "Agent"), NearToken::from_near(1)).await?);
    let spent = before - r.bal("carol").await;
    assert!(spent < NearToken::from_millinear(30).as_yoctonear(), "only storage is kept, rest refunded (spent {spent})");
    assert_ok(r.call("rover", "register", input("Rover-1", "Robot"), NearToken::from_near(1)).await?);

    let all: Vec<Value> = r.view("list_agents", json!({})).await?;
    assert_eq!(all.len(), 2);
    let robots: Vec<Value> = r.view("list_agents", json!({"kind": "Robot"})).await?;
    assert_eq!(robots.len(), 1);
    assert_eq!(robots[0]["account_id"], "rover.sandbox");
    let a: Value = r.view("get_agent", json!({"account_id": "carol.sandbox"})).await?;
    assert_eq!(a["strategy"]["min_score"], 70);
    Ok(())
}

#[tokio::test]
async fn t_registry_rules() -> TestResult<()> {
    let r = setup(&["carol", "mallory"]).await?;
    // too little deposit
    assert_err(&r.call("carol", "register", input("C", "Agent"), NearToken::from_yoctonear(1)).await?, "ERR_DEPOSIT");
    assert_ok(r.call("carol", "register", input("C", "Agent"), NearToken::from_near(1)).await?);
    // no duplicates; strangers cannot edit or remove carol's entry
    assert_err(&r.call("carol", "register", input("C2", "Agent"), NearToken::from_near(1)).await?, "ERR_ALREADY_REGISTERED");
    assert_err(&r.call("mallory", "update", input("Hacked", "Agent"), NearToken::from_near(1)).await?, "ERR_NOT_REGISTERED");
    assert_err(&r.call("mallory", "deregister", json!({}), NearToken::from_yoctonear(1)).await?, "ERR_NOT_REGISTERED");
    let a: Value = r.view("get_agent", json!({"account_id": "carol.sandbox"})).await?;
    assert_eq!(a["name"], "C");
    // pause hides from the default listing; deregister refunds the deposit
    assert_ok(r.call("carol", "set_active", json!({"active": false}), NearToken::from_yoctonear(0)).await?);
    let shown: Vec<Value> = r.view("list_agents", json!({})).await?;
    assert!(shown.is_empty());
    let before = r.bal("carol").await;
    assert_ok(r.call("carol", "deregister", json!({}), NearToken::from_yoctonear(1)).await?);
    assert!(r.bal("carol").await > before, "storage deposit refunded");
    let n: u32 = r.view("agents_count", json!({})).await?;
    assert_eq!(n, 0);
    Ok(())
}
