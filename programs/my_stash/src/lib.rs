use anchor_lang::{prelude::*, solana_program::clock};
use anchor_spl::token::{self, CloseAccount, SetAuthority, TokenAccount, Transfer};
use spl_token::instruction::AuthorityType;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod my_stash {
    use super::*;

    const MY_STASH_PDA_SEED: &[u8] = b"my_stash";

    pub fn initialize(ctx: Context<Initialize>, lock_seconds: u64) -> Result<()> {
        let stash_acc = &mut ctx.accounts.stash_account;
        let init_acc = &mut ctx.accounts.initializer;
        let stash_token_acc = &mut ctx.accounts.stash_token_account;
        stash_acc.initializer_key = *init_acc.key;
        stash_acc.stash_token_account = stash_token_acc.key();

        stash_acc.unlock_time = get_unlock_time(lock_seconds)?;

        let (stash_authority, _stash_authority_bump) =
            Pubkey::find_program_address(&[MY_STASH_PDA_SEED], ctx.program_id);

        token::set_authority(
            {
                let cpi_accounts = SetAuthority {
                    account_or_mint: stash_token_acc.to_account_info().clone(),
                    current_authority: init_acc.clone(),
                };
                CpiContext::new(ctx.accounts.token_program.clone(), cpi_accounts)
            },
            AuthorityType::AccountOwner,
            Some(stash_authority),
        )?;

        Ok(())
    }

    pub fn retrieve(ctx: Context<Retrieve>) -> Result<()> {
        let stash_acc = &mut ctx.accounts.stash_account;
        let stash_token_acc = &mut ctx.accounts.stash_token_account;
        let stash_token_acc_auth = &mut ctx.accounts.stash_token_account_authority;
        let reciever_token_acc = &mut ctx.accounts.reciever_token_account;

        // Check if the stash became unlocked
        if is_locked(stash_acc.unlock_time)? {
            panic!()
        }

        let (_stash_authority, stash_authority_bump) =
            Pubkey::find_program_address(&[MY_STASH_PDA_SEED], ctx.program_id);
        let authority_seeds = &[MY_STASH_PDA_SEED, &[stash_authority_bump]];

        token::transfer(
            {
                let cpi_accounts = Transfer {
                    from: stash_token_acc.to_account_info().clone(),
                    to: reciever_token_acc.to_account_info().clone(),
                    authority: stash_token_acc_auth.clone(),
                };
                CpiContext::new(ctx.accounts.token_program.clone(), cpi_accounts)
            }
            .with_signer(&[&authority_seeds[..]]),
            stash_token_acc.amount,
        )?;

        token::close_account(
            {
                let cpi_accounts = CloseAccount {
                    account: stash_token_acc.to_account_info().clone(),
                    destination: ctx.accounts.initializer.clone(),
                    authority: stash_token_acc_auth.clone(),
                };
                CpiContext::new(ctx.accounts.token_program.clone(), cpi_accounts)
            }
            .with_signer(&[&authority_seeds[..]]),
        )?;

        Ok(())
    }
}

#[account]
#[derive(Default)]
pub struct StashAccount {
    pub initializer_key: Pubkey,
    pub stash_token_account: Pubkey,
    pub unlock_time: i64,
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(mut, signer)]
    pub initializer: AccountInfo<'info>,
    #[account(init, payer = initializer, space = 8 + 32 * 2 + 8)]
    pub stash_account: Account<'info, StashAccount>,
    #[account(mut)]
    pub stash_token_account: Account<'info, TokenAccount>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub system_program: AccountInfo<'info>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub token_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct Retrieve<'info> {
    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(mut, signer)]
    pub initializer: AccountInfo<'info>,
    #[account(
        mut,
        constraint = stash_account.initializer_key == *initializer.key,
        constraint = stash_account.stash_token_account == *stash_token_account.to_account_info().key,
        close = initializer
    )]
    pub stash_account: Account<'info, StashAccount>,
    #[account(mut)]
    pub stash_token_account: Account<'info, TokenAccount>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub stash_token_account_authority: AccountInfo<'info>,
    #[account(mut)]
    pub reciever_token_account: Account<'info, TokenAccount>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub system_program: AccountInfo<'info>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub token_program: AccountInfo<'info>,
}

fn get_unlock_time(lock_seconds: u64) -> Result<i64> {
    let lock_seconds = lock_seconds as i64;
    if lock_seconds.is_negative() {
        panic!()
    }
    Ok(clock::Clock::get()?
        .unix_timestamp
        .checked_add(lock_seconds)
        .unwrap())
}

fn is_locked(unlock_time: i64) -> Result<bool> {
    Ok(clock::Clock::get()?.unix_timestamp < unlock_time)
}

