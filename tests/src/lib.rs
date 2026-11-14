#![allow(unused_must_use)]
//! Sandbox fixtures for FACTOR integration tests (near-sandbox + near-api, see DECISIONS.md).
//!
//! Each test gets its own sandbox with: musdc, the vault deployed as GLOBAL code (by hash),
//! factor, a real Sputnik DAO v2 (`dao.sandbox`, council bob/council2/council3, 2-of-3),
//! a plain payer account (`payer.sandbox`, used where governance isn't the point), alice, carol.
use near_api::types::transaction::result::ExecutionFinalResult;
use near_api::{AccountId, NearGas, NearToken, NetworkConfig, Signer};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::Digest;
use std::sync::Arc;

pub use testresult::TestResult;

pub const M: u128 = 1_000_000; // 1 mUSDC (6 decimals)
pub const FACE: u128 = 500 * M;
pub const DAY_NS: u64 = 86_400 * 1_000_000_000;

pub fn wasm(name: &str) -> Vec<u8> {
    let p = format!("{}/../target/near/{name}/{name}.wasm", env!("CARGO_MANIFEST_DIR"));
    std::fs::read(&p).unwrap_or_else(|_| panic!("missing {p}; run `make build` first"))
}

pub fn res(name: &str) -> Vec<u8> {
    let p = format!("{}/res/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read(&p).unwrap_or_else(|_| panic!("missing {p}"))
}

pub fn now_ns() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() as u64
}

pub fn id(s: &str) -> AccountId {
    s.parse().unwrap()
}

pub struct Env {
    pub sandbox: near_sandbox::Sandbox,
    pub net: NetworkConfig,
    pub signer: Arc<Signer>,
    pub factor: AccountId,
    pub musdc: AccountId,
    pub dao: AccountId,
    pub payer: AccountId,
    pub alice: AccountId,
    pub bob: AccountId,
    pub council2: AccountId,
    pub council3: AccountId,
    pub carol: AccountId,
    pub vault_hash: String,
}

/// Debug text of a tx result (used to look for panic messages).
pub fn text(r: &ExecutionFinalResult) -> String {
    format!("{r:?}")
}

#[track_caller]
pub fn assert_err(r: &ExecutionFinalResult, needle: &str) {
    let t = text(r);
    assert!(t.contains(needle), "expected failure containing {needle:?}, got:\n{t}");
}

#[track_caller]
pub fn assert_ok(r: ExecutionFinalResult) -> ExecutionFinalResult {
    let t = text(&r);
    assert!(r.clone().into_result().is_ok(), "expected success, got:\n{t}");
    r
}

impl Env {
    pub async fn new() -> TestResult<Env> {
        let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
        let net = NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
        let signer = Signer::from_secret_key(near_sandbox::config::DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY.parse()?)?;
        let mk = |s: &str| id(&format!("{s}.sandbox"));
        let mut env = Env {
            sandbox,
            net,
            signer,
            factor: mk("factor"),
            musdc: mk("musdc"),
            dao: mk("dao"),
            payer: mk("payer"),
            alice: mk("alice"),
            bob: mk("bob"),
            council2: mk("council2"),
            council3: mk("council3"),
            carol: mk("carol"),
            vault_hash: String::new(),
        };
        for a in [
            &env.factor, &env.musdc, &env.dao, &env.payer, &env.alice, &env.bob, &env.council2, &env.council3,
            &env.carol,
        ] {
            env.sandbox
                .create_account(a.as_str().parse()?)
                .initial_balance(NearToken::from_near(60))
                .send()
                .await?;
        }

        // mUSDC
        near_api::Contract::deploy(env.musdc.clone())
            .use_code(wasm("musdc"))
            .with_init_call("new", json!({}))?
            .with_signer(env.signer.clone())
            .send_to(&env.net)
            .await?
            .assert_success();

        // vault as GLOBAL code, referenced by hash
        let vault_code = wasm("factor_vault");
        env.vault_hash = bs58::encode(sha2::Sha256::digest(&vault_code)).into_string();
        near_api::Contract::deploy_global_contract_code(vault_code)
            .as_hash()
            .with_signer(env.factor.clone(), env.signer.clone())
            .send_to(&env.net)
            .await?
            .assert_success();

        // factor
        near_api::Contract::deploy(env.factor.clone())
            .use_code(wasm("factor"))
            .with_init_call(
                "new",
                json!({"owner_id": env.factor, "stablecoin": env.musdc, "vault_code_hash": env.vault_hash}),
            )?
            .with_signer(env.signer.clone())
            .send_to(&env.net)
            .await?
            .assert_success();

        // real Sputnik DAO v2 (default policy: council role, ratio 1/2 => 2 of 3 votes)
        near_api::Contract::deploy(env.dao.clone())
            .use_code(res("sputnikdao2.wasm"))
            .with_init_call(
                "new",
                json!({
                    "config": {"name": "marketing", "purpose": "FACTOR test DAO", "metadata": ""},
                    "policy": [env.bob, env.council2, env.council3],
                }),
            )?
            .with_signer(env.signer.clone())
            .send_to(&env.net)
            .await?
            .assert_success();

        // storage-register payers + alice, fund payers with mUSDC (carol registers per-test)
        for a in [env.dao.clone(), env.payer.clone(), env.alice.clone()] {
            env.register(&a).await?;
        }
        for a in [env.dao.clone(), env.payer.clone()] {
            let musdc = env.musdc.clone();
            assert_ok(env.call(&a, &musdc, "mint", json!({"account_id": a, "amount": (10_000 * M).to_string()}), 0, 30).await?);
        }
        Ok(env)
    }

    pub async fn call(
        &self,
        signer: &AccountId,
        contract: &AccountId,
        method: &str,
        args: Value,
        deposit_yocto: u128,
        tgas: u64,
    ) -> TestResult<ExecutionFinalResult> {
        let r = near_api::Contract(contract.clone())
            .call_function(method, args)
            .transaction()
            .deposit(NearToken::from_yoctonear(deposit_yocto))
            .gas(NearGas::from_tgas(tgas))
            .with_signer(signer.clone(), self.signer.clone())
            .send_to(&self.net)
            .await?;
        Ok(r.into_full().expect("final result"))
    }

    /// Several function calls to the same contract in ONE transaction (one receipt, run in order).
    /// State written by an earlier action is visible to the later ones; a failing action reverts all.
    pub async fn batch(
        &self,
        signer: &AccountId,
        contract: &AccountId,
        calls: Vec<(&str, Value, u128, u64)>,
    ) -> TestResult<ExecutionFinalResult> {
        use near_api::types::transaction::actions::Action;
        use near_api::types::transaction::actions::FunctionCallAction;
        let actions = calls
            .into_iter()
            .map(|(m, a, dep, tgas)| {
                Action::FunctionCall(Box::new(FunctionCallAction {
                    method_name: m.to_string(),
                    args: a.to_string().into_bytes(),
                    gas: NearGas::from_tgas(tgas),
                    deposit: NearToken::from_yoctonear(dep),
                }))
            })
            .collect();
        let r = near_api::Transaction::construct(signer.clone(), contract.clone())
            .add_actions(actions)
            .with_signer(self.signer.clone())
            .send_to(&self.net)
            .await?;
        Ok(r.into_full().expect("final result"))
    }

    pub async fn view<T: DeserializeOwned + Send + Sync>(&self, contract: &AccountId, method: &str, args: Value) -> TestResult<T> {
        Ok(near_api::Contract(contract.clone()).call_function(method, args).read_only().fetch_from(&self.net).await?.data)
    }

    pub async fn register(&self, who: &AccountId) -> TestResult<()> {
        let musdc = self.musdc.clone();
        assert_ok(
            self.call(who, &musdc, "storage_deposit", json!({"account_id": who}), NearToken::from_millinear(10).as_yoctonear(), 30)
                .await?,
        );
        Ok(())
    }

    pub async fn ft_balance(&self, who: &AccountId) -> TestResult<u128> {
        let v: String = self.view(&self.musdc, "ft_balance_of", json!({"account_id": who})).await?;
        Ok(v.parse()?)
    }

    pub async fn near_balance(&self, who: &AccountId) -> TestResult<NearToken> {
        Ok(near_api::Account(who.clone()).tokens().near_balance().fetch_from(&self.net).await?.total)
    }

    pub async fn invoice(&self, inv: &str) -> TestResult<Value> {
        self.view(&self.factor, "invoice", json!({"id": inv})).await
    }

    pub async fn status(&self, inv: &str) -> TestResult<String> {
        Ok(self.invoice(inv).await?["status"].as_str().unwrap().to_string())
    }

    pub async fn stats(&self, dao: &AccountId) -> TestResult<Value> {
        self.view(&self.factor, "payer_stats", json!({"dao": dao})).await
    }

    /// alice invoices `payer` for 500 mUSDC, due in `due_in_ns`. Returns the invoice id.
    pub async fn create(&self, payer: &AccountId, due_in_ns: u64) -> TestResult<String> {
        let factor = self.factor.clone();
        let r = assert_ok(
            self.call(
                &self.alice,
                &factor,
                "create_invoice",
                json!({"payer_dao": payer, "amount": FACE.to_string(), "due_ts": (now_ns() + due_in_ns).to_string(),
                       "memo": "Q4 design work"}),
                NearToken::from_millinear(100).as_yoctonear(),
                100,
            )
            .await?,
        );
        Ok(r.into_result().unwrap().json::<String>()?)
    }

    pub fn vault_of(&self, inv: &str) -> AccountId {
        id(&format!("inv-{inv}.{}", self.factor))
    }

    pub async fn accept_direct(&self, inv: &str) -> TestResult<()> {
        let factor = self.factor.clone();
        assert_ok(self.call(&self.payer, &factor, "accept_invoice", json!({"id": inv}), 0, 30).await?);
        Ok(())
    }

    /// `from` pays the vault of `inv` via ft_transfer_call.
    pub async fn pay(&self, from: &AccountId, inv: &str, amount: u128) -> TestResult<ExecutionFinalResult> {
        let musdc = self.musdc.clone();
        self.call(
            from,
            &musdc,
            "ft_transfer_call",
            json!({"receiver_id": self.vault_of(inv), "amount": amount.to_string(), "msg": ""}),
            1,
            150,
        )
        .await
    }

    pub async fn list(&self, owner: &AccountId, inv: &str, price: NearToken) -> TestResult<ExecutionFinalResult> {
        let factor = self.factor.clone();
        self.call(owner, &factor, "list", json!({"id": inv, "price": price.as_yoctonear().to_string()}), 1, 30).await
    }

    pub async fn buy(&self, buyer: &AccountId, inv: &str, deposit: NearToken) -> TestResult<ExecutionFinalResult> {
        let factor = self.factor.clone();
        self.call(buyer, &factor, "buy", json!({"id": inv}), deposit.as_yoctonear(), 50).await
    }

    pub async fn withdraw(&self, who: &AccountId, inv: &str) -> TestResult<ExecutionFinalResult> {
        let factor = self.factor.clone();
        self.call(who, &factor, "withdraw", json!({"id": inv}), 1, 150).await
    }

    // ------------------------------------------------------------ Sputnik governance
    /// Council member `bob` proposes a FunctionCall; bob + council2 approve => executes.
    /// Returns (proposal id, final vote tx result).
    pub async fn dao_function_call(
        &self,
        receiver: &AccountId,
        method: &str,
        args: Value,
        deposit_yocto: u128,
        tgas: u64,
        description: &str,
    ) -> TestResult<(u64, ExecutionFinalResult)> {
        use base64::Engine;
        let kind = json!({"FunctionCall": {
            "receiver_id": receiver,
            "actions": [{
                "method_name": method,
                "args": base64::engine::general_purpose::STANDARD.encode(args.to_string()),
                "deposit": deposit_yocto.to_string(),
                "gas": (tgas * 1_000_000_000_000).to_string(),
            }]
        }});
        let dao = self.dao.clone();
        let r = assert_ok(
            self.call(
                &self.bob,
                &dao,
                "add_proposal",
                json!({"proposal": {"description": description, "kind": kind}}),
                NearToken::from_near(1).as_yoctonear(), // proposal bond
                50,
            )
            .await?,
        );
        let pid: u64 = r.into_result().unwrap().json()?;
        let vote = json!({"id": pid, "action": "VoteApprove", "proposal": kind});
        assert_ok(self.call(&self.bob, &dao, "act_proposal", vote.clone(), 0, 300).await?);
        let p: Value = self.view(&dao, "get_proposal", json!({"id": pid})).await?;
        assert_eq!(p["status"], "InProgress", "1 of 3 votes must not execute");
        let last = self.call(&self.council2, &dao, "act_proposal", vote, 0, 300).await?;
        Ok((pid, last))
    }

    pub async fn proposal(&self, pid: u64) -> TestResult<Value> {
        self.view(&self.dao, "get_proposal", json!({"id": pid})).await
    }
}
