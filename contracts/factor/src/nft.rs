//! NEP-171 core / NEP-177 metadata / NEP-181 enumeration, with FACTOR rules:
//!  - no transfer while a withdraw (or a previous nft_transfer_call) is in flight (T5)
//!  - any transfer clears the listing (T12)
//!  - nft_transfer_call locks the invoice until nft_resolve_transfer runs, so the
//!    temporary receiver cannot withdraw and then bounce the token back
//!  - token metadata is computed live from the invoice + payer stats (FR-7)
use crate::{Factor, FactorExt, Status, views};
use near_contract_standards::non_fungible_token::core::{NonFungibleTokenCore, NonFungibleTokenResolver};
use near_contract_standards::non_fungible_token::enumeration::NonFungibleTokenEnumeration;
use near_contract_standards::non_fungible_token::metadata::{
    NFT_METADATA_SPEC, NFTContractMetadata, NonFungibleTokenMetadataProvider, TokenMetadata,
};
use near_contract_standards::non_fungible_token::{Token, TokenId};
use near_sdk::json_types::U128;
use near_sdk::{AccountId, PromiseOrValue, near, require, serde_json};
use std::collections::HashMap;

impl Factor {
    fn before_transfer(&mut self, token_id: &TokenId) {
        let inv = self.invoice_ref(token_id);
        require!(!inv.pending, "ERR_PENDING");
        self.listings.remove(token_id);
    }

    fn set_pending(&mut self, token_id: &TokenId, pending: bool) {
        let mut inv = self.invoice_ref(token_id).clone();
        inv.pending = pending;
        self.invoices.insert(token_id.clone(), inv);
    }

    pub(crate) fn decorate(&self, token: Token) -> Token {
        let Some(inv) = self.invoices.get(&token.token_id) else { return token };
        let view = self.invoice_view(inv, token.owner_id.clone());
        let stats = self.payer_stats(inv.payer_dao.clone());
        let status = match view.status {
            Status::Draft => "DRAFT",
            Status::Accepted => "ACCEPTED",
            Status::Paid => "PAID",
            Status::Settled => "SETTLED",
        };
        let description = format!(
            "{} mUSDC owed by {} · due {} · status {}{}{} · payer score {}/100",
            views::fmt_amount(inv.amount),
            inv.payer_dao,
            views::fmt_date(inv.due_ts),
            status,
            if view.is_listed { " (LISTED)" } else { "" },
            if view.is_overdue { " (OVERDUE)" } else { "" },
            stats.payer_score,
        );
        Token {
            metadata: Some(TokenMetadata {
                title: Some(format!("FACTOR Invoice #{}", inv.id)),
                description: Some(description),
                media: None,
                media_hash: None,
                copies: Some(1),
                issued_at: Some((inv.created_ts / 1_000_000).to_string()),
                expires_at: None,
                starts_at: None,
                updated_at: None,
                extra: serde_json::to_string(&view).ok(),
                reference: None,
                reference_hash: None,
            }),
            ..token
        }
    }
}

#[near]
impl NonFungibleTokenCore for Factor {
    #[payable]
    fn nft_transfer(&mut self, receiver_id: AccountId, token_id: TokenId, approval_id: Option<u64>, memo: Option<String>) {
        self.before_transfer(&token_id);
        self.tokens.nft_transfer(receiver_id, token_id, approval_id, memo)
    }

    #[payable]
    fn nft_transfer_call(
        &mut self,
        receiver_id: AccountId,
        token_id: TokenId,
        approval_id: Option<u64>,
        memo: Option<String>,
        msg: String,
    ) -> PromiseOrValue<bool> {
        self.before_transfer(&token_id);
        self.set_pending(&token_id, true);
        self.tokens.nft_transfer_call(receiver_id, token_id, approval_id, memo, msg)
    }

    fn nft_token(&self, token_id: TokenId) -> Option<Token> {
        self.tokens.nft_token(token_id).map(|t| self.decorate(t))
    }
}

#[near]
impl NonFungibleTokenResolver for Factor {
    #[private]
    fn nft_resolve_transfer(
        &mut self,
        previous_owner_id: AccountId,
        receiver_id: AccountId,
        token_id: TokenId,
        approved_account_ids: Option<HashMap<AccountId, u64>>,
    ) -> bool {
        if self.invoices.contains_key(&token_id) {
            self.set_pending(&token_id, false);
        }
        self.tokens.nft_resolve_transfer(previous_owner_id, receiver_id, token_id, approved_account_ids)
    }
}

#[near]
impl NonFungibleTokenEnumeration for Factor {
    fn nft_total_supply(&self) -> U128 {
        self.tokens.nft_total_supply()
    }

    fn nft_tokens(&self, from_index: Option<U128>, limit: Option<u64>) -> Vec<Token> {
        self.tokens.nft_tokens(from_index, limit).into_iter().map(|t| self.decorate(t)).collect()
    }

    fn nft_supply_for_owner(&self, account_id: AccountId) -> U128 {
        self.tokens.nft_supply_for_owner(account_id)
    }

    fn nft_tokens_for_owner(&self, account_id: AccountId, from_index: Option<U128>, limit: Option<u64>) -> Vec<Token> {
        self.tokens
            .nft_tokens_for_owner(account_id, from_index, limit)
            .into_iter()
            .map(|t| self.decorate(t))
            .collect()
    }
}

#[near]
impl NonFungibleTokenMetadataProvider for Factor {
    fn nft_metadata(&self) -> NFTContractMetadata {
        NFTContractMetadata {
            spec: NFT_METADATA_SPEC.to_string(),
            name: "FACTOR Invoices".to_string(),
            symbol: "FACTOR".to_string(),
            icon: None,
            base_uri: None,
            reference: None,
            reference_hash: None,
        }
    }
}
