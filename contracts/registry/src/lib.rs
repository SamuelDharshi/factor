//! FACTOR registry: AI agents and robots register on NEAR and publish a strategy.
//!
//! An entry belongs to the NEAR account that registered it (one entry per account). The
//! registrant pays the storage it uses (refunded on deregister); only the owner can change it.
//! An agent's strategy (min payer score, min implied APR, max price) is public, so humans and
//! other agents can see how it will behave when it buys FACTOR invoices from its own account.
use near_sdk::json_types::{U64, U128};
use near_sdk::serde_json::json;
use near_sdk::store::IterableMap;
use near_sdk::{AccountId, BorshStorageKey, NearToken, PanicOnDefault, Promise, env, near, require};

pub const MAX_NAME: usize = 40;
pub const MAX_DESCRIPTION: usize = 280;
pub const MAX_ENDPOINT: usize = 200;
pub const MAX_CAPABILITIES: usize = 8;
pub const MAX_CAPABILITY_LEN: usize = 24;
/// Overhead of the map entry (key + bookkeeping) counted on top of the serialized value.
const ENTRY_OVERHEAD_BYTES: u128 = 128;

#[derive(BorshStorageKey)]
#[near]
enum StorageKey {
    Agents,
}

#[near(serializers = [borsh, json])]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Agent,
    Robot,
}

/// How the agent decides to buy an invoice. Enforced by the agent's own code, published here.
#[near(serializers = [borsh, json])]
#[derive(Clone, Debug, PartialEq)]
pub struct Strategy {
    /// Only buy invoices whose payer score is at least this (0-100).
    pub min_score: u32,
    /// Only buy at an implied APR of at least this percent.
    pub min_apr_pct: u32,
    /// Never pay more than this many yoctoNEAR for one invoice.
    pub max_price: U128,
}

#[near(serializers = [borsh, json])]
#[derive(Clone, Debug)]
pub struct Agent {
    pub account_id: AccountId,
    pub name: String,
    pub kind: Kind,
    pub description: String,
    pub capabilities: Vec<String>,
    /// Where the agent/robot can be reached (URL, MCP endpoint, device id...). Free text.
    pub endpoint: String,
    pub strategy: Strategy,
    pub registered_ts: U64,
    pub active: bool,
    /// Storage deposit held for this entry (yoctoNEAR); refunded on deregister.
    pub storage_deposit: U128,
}

#[near(serializers = [json])]
pub struct AgentInput {
    pub name: String,
    pub kind: Kind,
    pub description: String,
    pub capabilities: Vec<String>,
    pub endpoint: String,
    pub strategy: Strategy,
}

#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct Registry {
    agents: IterableMap<AccountId, Agent>,
}

fn validate(i: &AgentInput) {
    let name = i.name.trim();
    require!(!name.is_empty() && i.name.len() <= MAX_NAME, "ERR_NAME");
    require!(i.description.len() <= MAX_DESCRIPTION, "ERR_DESCRIPTION_TOO_LONG");
    require!(i.endpoint.len() <= MAX_ENDPOINT, "ERR_ENDPOINT_TOO_LONG");
    require!(i.capabilities.len() <= MAX_CAPABILITIES, "ERR_TOO_MANY_CAPABILITIES");
    require!(
        i.capabilities.iter().all(|c| !c.trim().is_empty() && c.len() <= MAX_CAPABILITY_LEN),
        "ERR_CAPABILITY"
    );
    require!(i.strategy.min_score <= 100, "ERR_MIN_SCORE");
    require!(i.strategy.min_apr_pct <= 10_000, "ERR_MIN_APR");
}

fn required_deposit(a: &Agent) -> NearToken {
    let bytes = near_sdk::borsh::to_vec(a).expect("serialize").len() as u128 + ENTRY_OVERHEAD_BYTES;
    env::storage_byte_cost().saturating_mul(bytes)
}

fn emit(event: &str, data: near_sdk::serde_json::Value) {
    let ev = json!({"standard": "factor_registry", "version": "1.0.0", "event": event, "data": [data]});
    env::log_str(&format!("EVENT_JSON:{ev}"));
}

#[near]
impl Registry {
    #[init]
    pub fn new() -> Self {
        Self { agents: IterableMap::new(StorageKey::Agents) }
    }

    /// Register the caller as an agent or robot. Pays for its own storage; excess is refunded.
    #[payable]
    pub fn register(&mut self, input: AgentInput) -> Agent {
        let who = env::predecessor_account_id();
        require!(!self.agents.contains_key(&who), "ERR_ALREADY_REGISTERED");
        validate(&input);
        let mut agent = Agent {
            account_id: who.clone(),
            name: input.name.trim().to_string(),
            kind: input.kind,
            description: input.description,
            capabilities: input.capabilities.into_iter().map(|c| c.trim().to_string()).collect(),
            endpoint: input.endpoint,
            strategy: input.strategy,
            registered_ts: U64(env::block_timestamp()),
            active: true,
            storage_deposit: U128(0),
        };
        let required = required_deposit(&agent);
        let attached = env::attached_deposit();
        require!(
            attached >= required,
            format!("ERR_DEPOSIT: attach at least {} yoctoNEAR", required.as_yoctonear())
        );
        agent.storage_deposit = U128(required.as_yoctonear());
        let excess = attached.saturating_sub(required);
        if !excess.is_zero() {
            let _ = Promise::new(who.clone()).transfer(excess);
        }
        emit("agent_registered", json!({"account_id": who, "name": agent.name, "kind": agent.kind}));
        self.agents.insert(who, agent.clone());
        agent
    }

    /// Owner only. Replaces the entry's details; storage is re-priced (pay growth / get refund).
    #[payable]
    pub fn update(&mut self, input: AgentInput) -> Agent {
        let who = env::predecessor_account_id();
        let old = self.agents.get(&who).cloned().unwrap_or_else(|| env::panic_str("ERR_NOT_REGISTERED"));
        validate(&input);
        let mut agent = Agent {
            name: input.name.trim().to_string(),
            kind: input.kind,
            description: input.description,
            capabilities: input.capabilities.into_iter().map(|c| c.trim().to_string()).collect(),
            endpoint: input.endpoint,
            strategy: input.strategy,
            ..old.clone()
        };
        let required = required_deposit(&agent).as_yoctonear();
        let held = old.storage_deposit.0;
        let attached = env::attached_deposit().as_yoctonear();
        let refund = if required > held {
            let extra = required - held;
            require!(attached >= extra, format!("ERR_DEPOSIT: attach at least {extra} yoctoNEAR"));
            attached - extra
        } else {
            attached + (held - required)
        };
        agent.storage_deposit = U128(required);
        if refund > 0 {
            let _ = Promise::new(who.clone()).transfer(NearToken::from_yoctonear(refund));
        }
        emit("agent_updated", json!({"account_id": who}));
        self.agents.insert(who, agent.clone());
        agent
    }

    /// Owner only. Pause or resume the agent in the marketplace without losing the entry.
    pub fn set_active(&mut self, active: bool) {
        let who = env::predecessor_account_id();
        let mut a = self.agents.get(&who).cloned().unwrap_or_else(|| env::panic_str("ERR_NOT_REGISTERED"));
        a.active = active;
        self.agents.insert(who.clone(), a);
        emit("agent_active", json!({"account_id": who, "active": active}));
    }

    /// Owner only. Removes the entry and refunds the storage deposit.
    #[payable]
    pub fn deregister(&mut self) -> Promise {
        near_sdk::assert_one_yocto();
        let who = env::predecessor_account_id();
        let a = self.agents.remove(&who).unwrap_or_else(|| env::panic_str("ERR_NOT_REGISTERED"));
        emit("agent_deregistered", json!({"account_id": who}));
        Promise::new(who).transfer(NearToken::from_yoctonear(a.storage_deposit.0 + 1))
    }

    // ------------------------------------------------------------------ views
    pub fn get_agent(&self, account_id: AccountId) -> Option<Agent> {
        self.agents.get(&account_id).cloned()
    }

    pub fn agents_count(&self) -> u32 {
        self.agents.len()
    }

    /// Newest registrations last. `kind` filters; `active_only` hides paused entries.
    pub fn list_agents(
        &self,
        from_index: Option<u32>,
        limit: Option<u32>,
        kind: Option<Kind>,
        active_only: Option<bool>,
    ) -> Vec<Agent> {
        let active_only = active_only.unwrap_or(true);
        self.agents
            .values()
            .filter(|a| kind.is_none_or(|k| a.kind == k) && (!active_only || a.active))
            .skip(from_index.unwrap_or(0) as usize)
            .take(limit.unwrap_or(50).min(100) as usize)
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::VMContextBuilder;
    use near_sdk::testing_env;

    fn acc(s: &str) -> AccountId {
        s.parse().unwrap()
    }
    fn ctx(who: &str, deposit: NearToken) {
        testing_env!(
            VMContextBuilder::new()
                .current_account_id(acc("registry.testnet"))
                .predecessor_account_id(acc(who))
                .attached_deposit(deposit)
                .block_timestamp(1_800_000_000_000_000_000)
                .build()
        );
    }
    fn input(name: &str, kind: Kind) -> AgentInput {
        AgentInput {
            name: name.into(),
            kind,
            description: "buys high-score invoices".into(),
            capabilities: vec!["buy-invoices".into(), "risk-scoring".into()],
            endpoint: "https://agent.example/mcp".into(),
            strategy: Strategy { min_score: 70, min_apr_pct: 40, max_price: U128(10_000_000_000_000_000_000_000_000) },
        }
    }
    fn one_near() -> NearToken {
        NearToken::from_near(1)
    }

    #[test]
    fn register_and_view() {
        ctx("carol.testnet", one_near());
        let mut r = Registry::new();
        let a = r.register(input("  Carol Capital ", Kind::Agent));
        assert_eq!(a.name, "Carol Capital", "name is trimmed");
        assert!(a.active && a.storage_deposit.0 > 0);
        assert!(a.storage_deposit.0 < NearToken::from_millinear(20).as_yoctonear(), "cheap: {}", a.storage_deposit.0);
        assert_eq!(r.agents_count(), 1);
        assert_eq!(r.get_agent(acc("carol.testnet")).unwrap().strategy.min_score, 70);
        assert!(r.get_agent(acc("bob.testnet")).is_none());
    }

    #[test]
    #[should_panic(expected = "ERR_DEPOSIT")]
    fn register_requires_storage_deposit() {
        ctx("carol.testnet", NearToken::from_yoctonear(1));
        Registry::new().register(input("x", Kind::Agent));
    }

    #[test]
    #[should_panic(expected = "ERR_ALREADY_REGISTERED")]
    fn one_entry_per_account() {
        ctx("carol.testnet", one_near());
        let mut r = Registry::new();
        r.register(input("a", Kind::Agent));
        r.register(input("b", Kind::Robot));
    }

    #[test]
    fn validation_rules() {
        for (bad, msg) in [
            (AgentInput { name: "   ".into(), ..input("x", Kind::Agent) }, "ERR_NAME"),
            (AgentInput { name: "n".repeat(MAX_NAME + 1), ..input("x", Kind::Agent) }, "ERR_NAME"),
            (AgentInput { description: "d".repeat(MAX_DESCRIPTION + 1), ..input("x", Kind::Agent) }, "ERR_DESCRIPTION_TOO_LONG"),
            (AgentInput { capabilities: vec!["c".into(); MAX_CAPABILITIES + 1], ..input("x", Kind::Agent) }, "ERR_TOO_MANY_CAPABILITIES"),
            (AgentInput { capabilities: vec!["".into()], ..input("x", Kind::Agent) }, "ERR_CAPABILITY"),
            (AgentInput { strategy: Strategy { min_score: 101, min_apr_pct: 0, max_price: U128(1) }, ..input("x", Kind::Agent) }, "ERR_MIN_SCORE"),
        ] {
            ctx("carol.testnet", one_near());
            let res = std::panic::catch_unwind(|| {
                let mut r = Registry::new();
                r.register(bad);
            });
            let err = res.expect_err("must reject");
            let text = err.downcast_ref::<String>().cloned().or_else(|| err.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default();
            assert!(text.contains(msg), "expected {msg}, got {text}");
        }
    }

    #[test]
    fn update_only_own_entry_and_reprices_storage() {
        ctx("carol.testnet", one_near());
        let mut r = Registry::new();
        let before = r.register(input("Carol", Kind::Agent)).storage_deposit.0;
        let mut longer = input("Carol v2", Kind::Agent);
        longer.description = "d".repeat(MAX_DESCRIPTION);
        ctx("carol.testnet", one_near());
        let after = r.update(longer).storage_deposit.0;
        assert!(after > before, "longer text costs more storage");
        ctx("carol.testnet", NearToken::from_yoctonear(0));
        let shrunk = r.update(input("Carol", Kind::Agent)).storage_deposit.0;
        assert_eq!(shrunk, before);
    }

    #[test]
    #[should_panic(expected = "ERR_NOT_REGISTERED")]
    fn stranger_cannot_update_someone_elses_entry() {
        ctx("carol.testnet", one_near());
        let mut r = Registry::new();
        r.register(input("Carol", Kind::Agent));
        ctx("mallory.testnet", one_near());
        r.update(input("Hacked", Kind::Agent)); // keyed by caller: mallory has no entry
    }

    #[test]
    fn pause_filter_and_deregister() {
        ctx("carol.testnet", one_near());
        let mut r = Registry::new();
        r.register(input("Carol", Kind::Agent));
        ctx("rover.testnet", one_near());
        r.register(input("Rover", Kind::Robot));
        assert_eq!(r.list_agents(None, None, None, None).len(), 2);
        assert_eq!(r.list_agents(None, None, Some(Kind::Robot), None).len(), 1);
        ctx("rover.testnet", NearToken::from_yoctonear(0));
        r.set_active(false);
        assert_eq!(r.list_agents(None, None, None, None).len(), 1, "paused hidden by default");
        assert_eq!(r.list_agents(None, None, None, Some(false)).len(), 2);
        ctx("rover.testnet", NearToken::from_yoctonear(1));
        let _ = r.deregister();
        assert_eq!(r.agents_count(), 1);
        assert!(r.get_agent(acc("rover.testnet")).is_none());
    }

    #[test]
    #[should_panic(expected = "Requires attached deposit of exactly 1 yoctoNEAR")]
    fn deregister_needs_one_yocto() {
        ctx("carol.testnet", one_near());
        let mut r = Registry::new();
        r.register(input("Carol", Kind::Agent));
        ctx("carol.testnet", NearToken::from_yoctonear(0));
        let _ = r.deregister();
    }

    #[test]
    fn pagination() {
        let mut r = Registry::new();
        for i in 0..5 {
            let who = format!("a{i}.testnet");
            ctx(&who, one_near());
            r.register(input(&format!("A{i}"), Kind::Agent));
        }
        assert_eq!(r.list_agents(Some(1), Some(2), None, None).len(), 2);
        assert_eq!(r.list_agents(Some(4), Some(10), None, None).len(), 1);
    }
}
