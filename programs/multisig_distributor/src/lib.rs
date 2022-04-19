use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{Instruction};
use anchor_lang::solana_program::program::{invoke_signed, invoke};
use anchor_spl::token::{Token, Mint, TokenAccount, Transfer};
pub mod merkle_proof;
use vipers::prelude::*;


declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod multisig_distributor {
    use super::*;

    pub fn create_multisig(
        ctx: Context<CreateMultisig>,
        owners: Vec<Pubkey>,
        threshold: u64,
        nonce: u8,) -> Result<()> {
            assert_unique_owners(owners);
            require!(threshold > 0 &&  threshold <= owners.len() as u64, Errors::InvalidThreshold);
            require!(!owners.is_empty(), Errors::InvalidOwnersLen);
            let multisig = &mut ctx.accounts.multisig;
            multisig.owners = owners;
            multisig.threshold = threshold;
            multisig.nonce = nonce;
            multisig.owner_set_seqno = 0;

        Ok(())
    }
    pub fn create_transaction(
        ctx: Context<CreateTransaction>,
        pid: Pubkey,
        accs: Vec<TransactionAccount>,
        data: Vec<u8>,) -> Result<()> {
            let owner_index = ctx.accounts.multisig.owners.iter().position(|a| a == ctx.accounts.proposer.key).ok_or(Errors::InvalidOwner);
            

        Ok(())
    }
    pub fn new_distributor(
        ctx: Context<NewDistributor>,
        root: [u8; 32],
        _bump: u8,
        max_tot_claim: u64,
        max_num_nodes: u64,
     ) -> Result<()> {
        let owner_index = ctx
            .accounts
            .multisig
            .owners
            .iter()
            .position(|a| a == ctx.accounts.owner.key)
            .ok_or(Errors::InvalidOwner)?;

        ctx.accounts.transaction.signers[owner_index] = true;

        // a check to ensure that there are enough signers.
        let sig_count = ctx
        .accounts
        .transaction
        .signers
        .iter()
        .filter(|&did_sign| *did_sign)
        .count() as u64;
        if sig_count < ctx.accounts.multisig.threshold {
            return Err(Errors::NotEnoughSigners.into());
        }
        else {
            let distributor = &mut ctx.accounts.distributor;
            distributor.multisig_base = *ctx.accounts.multisig_base.to_account_info().key;
            distributor.bump = _bump;
            distributor.root = root;
            distributor.max_num_nodes = max_num_nodes;
            distributor.max_total_claim = max_tot_claim;
            distributor.mint = ctx.accounts.mint.key();
            distributor.total_claimed = 0;
            distributor.num_nodes_claimed = 0;
        }
        Ok(())
    }
    pub fn approve_and_execute_tx(ctx: Context<ApproveExecute>) -> Result<()>{
            let owner_index = ctx
            .accounts
            .multisig
            .owners
            .iter()
            .position(|a| a == ctx.accounts.owner.key)
            .ok_or(Errors::InvalidOwner)?;

        // sets that owner to as approved
        ctx.accounts.transaction.signers[owner_index] = true;

        // checks if threshold has met
        let sig_count = ctx
            .accounts
            .transaction
            .signers 
            .iter()
            .filter(|&did_sign| *did_sign)
            .count() as u64;

        if sig_count < ctx.accounts.multisig.threshold {
            return Ok(());
        }

        // return if transaction already executed
        if ctx.accounts.transaction.did_execute {
            return Err(Errors::AlreadyExecuted.into());
        }

        let mut ixs: Vec<Instruction> = (&*ctx.accounts.transaction).into();

        for ix in ixs.iter_mut() {
            ix.accounts = ix
                .accounts
                .iter()
                .map(|acc| {
                    let mut acc = acc.clone();
                    if &acc.pubkey == ctx.accounts.multisig_signer.key {
                        acc.is_signer = true;
                    }
                    acc
                })
                .collect();
        }

        let seeds = &[
            ctx.accounts.multisig.to_account_info().key.as_ref(),
            &[ctx.accounts.multisig.nonce],
        ];

        let signer = &[&seeds[..]]; //&[&[&[u8]]];
        let accounts = ctx.remaining_accounts;
        for ix in ixs.iter() {
            invoke_signed(ix, &accounts, signer)?;
        }

        ctx.accounts.transaction.did_execute = true;
    
        Ok(())
    }
    pub fn claim(
        ctx: Context<Claim>, 
        index: u64, 
        bump: u8, 
        proof: Vec<[u8; 32]>,
        amount: u64) ->Result<()>{
            let claim_status = &mut ctx.accounts.claim_status;
            invariant!(!claim_status.is_claimed && claim_status.claimed_at == 0,
            Errors::DropAlreadyClaimed)
        }
}

#[derive(Accounts)]
pub struct CreateMultisig<'info>{
    #[account(zero, signer)]
    multisig: Box<Account<'info, Multisig>>,
    rent: Sysvar<'info, Rent>,
}
#[derive(Accounts)]
pub struct CreateTransaction<'info>{
    multisig: Box<Account<'info, Multisig>>,
    #[account(zero, signer)]
    transaction: Box<Account<'info, Transaction>>,
    proposer: Signer<'info>,

}
#[derive(Accounts)]
pub struct NewDistributor<'info>{
    #[account(mut)]
    multisig_base: Signer<'info>,

    #[account(
        init, 
        seeds = [ b"MerkleDistributor".as_ref(),
        multisig_base.key().to_bytes().as_ref()],
        bump,
        payer = multisig_base)]
    distributor: Account<'info, Distributor>,
    mint: Account<'info, Mint>,
    transaction: Account<'info, Transaction>,
    multisig: Box<Account<'info, Multisig>>,
    rent: Sysvar<'info, Rent>,
    system_program: Program<'info, System>,
}


#[derive(Accounts)]
pub struct ApproveExecute<'info>{
    #[account(constraint = multisig.owner_set_seqno == transaction.owner_set_seqno)]
    pub multisig: Account<'info, Multisig>,

    #[account(
        seeds = [multisig.to_account_info().key.as_ref()],
        bump = multisig.nonce,
    )]
    pub multisig_signer: AccountInfo<'info>,

    #[account(mut, has_one = multisig)]
    pub transaction: Account<'info, Transaction>,

    #[account(signer)]
    pub owner: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct Claim<'info>{
   
   #[account(
       init,
        seeds = [
            b"ClaimStatus".as_ref(),
            index.to_le_bytes().as_ref(),
            distributor.key().to_bytes().as_ref()
        ],
        bump,
        payer = payer)]
   claim_status: Account<'info, ClaimStatus>,

   #[account(mut)]
   distributor: Account<'info, Distributor>,

   #[account(mut)]
   payer: Signer<'info>,

   claimant: Signer<'info>,

   #[account(mut)]
   from: Account<'info, TokenAccount>,
   
   #[account(mut)] 
   to: Account<'info, TokenAccount>,
   system_program: Program<'info, System>,
   token_program: Program<'info, Token>,
}





fn assert_unique_owners(owners: Vec<Pubkey>) -> Result<()>{
    for (i,owner) in owners.iter().enumerate(){
        require!(
            !owners.iter().skip(i + 1).any(|item| item == owner),
            Errors::UniqueOwners
        )
    }
    Ok(())
}
#[account]
pub struct Multisig{
  pub owners: Vec<Pubkey>,
  pub threshold: u64,
  pub nonce: u8,
  pub owner_set_seqno: u32,
}

#[account]
pub struct Transaction{
    pub multisig: Pubkey,
    pub program_ids: Vec<Pubkey>,
    pub accounts: Vec<Vec<TransactionAccount>>,
    pub data: Vec<Vec<u8>>,
    pub signers: Vec<bool>,
    pub did_execute: bool,
    pub owner_set_seqno: u32,
}
impl From<Transaction> for Vec<Instruction>{
    fn from(tx: Transaction) -> Vec<Instruction>{
        let mut instructions: Vec<Instruction> = Vec::new();
        for (i, _p_ids) in tx.program_ids.iter().enumerate(){
            instructions.push(
                Instruction{
                    program_id: tx.program_ids[i],
                    accounts: tx.accounts[i].iter().map(|t| AccountMeta::from(t)).collect(),
                    data: tx.data[i].clone(),
                }
            )
        }
        instructions
    }
}


#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct TransactionAccount {
    pub pubkey: Pubkey,
    pub is_signer: bool,
    pub is_writable: bool,
}

impl From<&AccountMeta> for TransactionAccount {
    fn from(account_meta: &AccountMeta) -> TransactionAccount {
        TransactionAccount {
            pubkey: account_meta.pubkey,
            is_signer: account_meta.is_signer,
            is_writable: account_meta.is_writable,
        }
    }
}

#[account]
#[derive(Default)]
pub struct Distributor{
    //multisig base key to derive the PDA,
    pub multisig_base: Pubkey,
    // bump seed of the pda,
    pub bump: u8,
    //merkle root 
    pub root: [u8; 32],
    //mint supported by the distributor,
    pub mint: Pubkey,
    pub max_total_claim: u64,
    pub max_num_nodes: u64,
    pub total_claimed: u64,
    pub num_nodes_claimed: u64,
}
#[account]
pub struct ClaimStatus{
    is_claimed: bool,
    claimant: Pubkey,
    amount: u64,
    claimed_at: i64,
}
#[event]
pub struct Claimedevent{
    index: u64,
    claimant: Pubkey,
    amount: u64,
}
#[error_code]
pub enum Errors{
    #[msg("Owners must be unique")]
    UniqueOwners,
    #[msg("Invalid no of owners")]
    InvalidOwnersLen,
    #[msg("Invalid owner")]
    InvalidOwner,
    #[msg("Threshold doesn't meet the requirement")]
    InvalidThreshold,
    #[msg("drop isalready claimed")]
    DropAlreadyClaimed,

}