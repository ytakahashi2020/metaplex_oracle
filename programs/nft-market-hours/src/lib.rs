use anchor_lang::prelude::*;
use anchor_lang::solana_program::system_instruction;
use anchor_lang::solana_program::system_program;
use anchor_lang::AnchorDeserialize;
use anchor_lang::AnchorSerialize;

declare_id!("32RNH2JPGUCGdYp5TetkT1Y2CHwAUk2XzXn6VCFqvb7F");

#[program]
pub mod nft_market_hours {
    use super::*;

    // Oracleの作成
    pub fn create_oracle(ctx: Context<CreateOracle>) -> Result<()> {
        msg!("Creating Oracle account...");
        let unix_timestamp = Clock::get()?.unix_timestamp;

        // 市場が開いているか確認
        let is_market_open = is_us_market_open(unix_timestamp);

        msg!("Setting Oracle state...");
        // Oracleアカウントに保存する情報を設定
        ctx.accounts.oracle.set_inner(Oracle {
            validation: if is_market_open {
                OracleValidation::V1 {
                    transfer: ExternalValidationResult::Approved,
                    create: ExternalValidationResult::Pass,
                    update: ExternalValidationResult::Pass,
                    burn: ExternalValidationResult::Pass,
                }
            } else {
                OracleValidation::V1 {
                    transfer: ExternalValidationResult::Rejected,
                    create: ExternalValidationResult::Pass,
                    update: ExternalValidationResult::Pass,
                    burn: ExternalValidationResult::Pass,
                }
            },
            bump: ctx.bumps.oracle,
            vault_bump: ctx.bumps.reward_vault,
        });

        msg!("Oracle account created successfully.");
        Ok(())
    }

    // Oracleのデータ更新（Crank）
    pub fn crank_oracle(ctx: Context<CrankOracle>) -> Result<()> {
        let unix_timestamp = Clock::get()?.unix_timestamp;
        let is_market_open = is_us_market_open(unix_timestamp);

        // Oracleの状態を更新
        if is_market_open {
            ctx.accounts.oracle.validation = OracleValidation::V1 {
                transfer: ExternalValidationResult::Approved,
                create: ExternalValidationResult::Pass,
                burn: ExternalValidationResult::Pass,
                update: ExternalValidationResult::Pass,
            };
        } else {
            ctx.accounts.oracle.validation = OracleValidation::V1 {
                transfer: ExternalValidationResult::Rejected,
                create: ExternalValidationResult::Pass,
                burn: ExternalValidationResult::Pass,
                update: ExternalValidationResult::Pass,
            };
        }

        // 報酬の支払い処理
        let reward_vault_lamports = ctx.accounts.reward_vault.lamports();
        let oracle_key = ctx.accounts.oracle.key().clone();
        let signer_seeds = &[
            b"reward_vault",
            oracle_key.as_ref(),
            &[ctx.accounts.oracle.bump],
        ];

        if is_within_15_minutes_of_market_open_or_close(unix_timestamp)
            && reward_vault_lamports > REWARD_IN_LAMPORTS
        {
            // 報酬の送金処理
            let transfer_instruction = system_instruction::transfer(
                &ctx.accounts.reward_vault.key(),
                &ctx.accounts.signer.key(),
                REWARD_IN_LAMPORTS,
            );
            anchor_lang::solana_program::program::invoke_signed(
                &transfer_instruction,
                &[
                    ctx.accounts.reward_vault.to_account_info(),
                    ctx.accounts.signer.to_account_info(),
                    ctx.accounts.system_program.to_account_info(),
                ],
                &[signer_seeds],
            )?;
        }

        Ok(())
    }
}

// Oracleアカウント
#[account]
pub struct Oracle {
    pub validation: OracleValidation,
    pub bump: u8,
    pub vault_bump: u8,
}

impl Space for Oracle {
    const INIT_SPACE: usize = 8 + 30;
}

// Oracle Validation
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub enum OracleValidation {
    V1 {
        transfer: ExternalValidationResult,
        create: ExternalValidationResult,
        update: ExternalValidationResult,
        burn: ExternalValidationResult,
    },
}

// External Validationの結果
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub enum ExternalValidationResult {
    Approved,
    Rejected,
    Pass,
}

// CreateOracleアカウント
#[derive(Accounts)]
pub struct CreateOracle<'info> {
    pub signer: Signer<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init,  // アカウントを初期化する
        payer = payer,  // 支払いをするアカウント
        space = Oracle::INIT_SPACE,  // Oracleアカウントに必要なスペースを定義
        seeds = [b"oracle"],
        bump
    )]
    pub oracle: Account<'info, Oracle>, // Oracleアカウント
    #[account(
        seeds = [b"reward_vault", oracle.key().as_ref()],
        bump,
    )]
    pub reward_vault: SystemAccount<'info>, // 報酬を格納するアカウント
    pub system_program: Program<'info, System>, // システムプログラム
}

// CrankOracleアカウント
#[derive(Accounts)]
pub struct CrankOracle<'info> {
    pub signer: Signer<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        mut,
        seeds = [b"oracle"],
        bump = oracle.bump,
    )]
    pub oracle: Account<'info, Oracle>,
    #[account(
        mut, 
        seeds = [b"reward_vault", oracle.key().as_ref()],
        bump = oracle.vault_bump,
    )]
    pub reward_vault: SystemAccount<'info>,
    pub system_program: Program<'info, System>,
}

// US市場が開いているか確認するヘルパー関数
fn is_us_market_open(unix_timestamp: i64) -> bool {
    let seconds_since_midnight = unix_timestamp % SECONDS_IN_A_DAY;
    let weekday = (unix_timestamp / SECONDS_IN_A_DAY + 4) % 7;

    if weekday >= 5 {
        return false;
    }

    seconds_since_midnight >= MARKET_OPEN_TIME && seconds_since_midnight < MARKET_CLOSE_TIME
}

// US市場の開閉15分前後か確認するヘルパー関数
fn is_within_15_minutes_of_market_open_or_close(unix_timestamp: i64) -> bool {
    let seconds_since_midnight = unix_timestamp % SECONDS_IN_A_DAY;

    (seconds_since_midnight >= MARKET_OPEN_TIME
        && seconds_since_midnight < MARKET_OPEN_TIME + MARKET_OPEN_CLOSE_MARGIN)
        || (seconds_since_midnight >= MARKET_CLOSE_TIME
            && seconds_since_midnight < MARKET_CLOSE_TIME + MARKET_OPEN_CLOSE_MARGIN)
}

// 定数
const SECONDS_IN_A_DAY: i64 = 86400;
const MARKET_OPEN_TIME: i64 = 14 * 3600 + 30 * 60; // 14:30 UTC = 9:30 EST
const MARKET_CLOSE_TIME: i64 = 21 * 3600; // 21:00 UTC = 16:00 EST
const MARKET_OPEN_CLOSE_MARGIN: i64 = 15 * 60; // 15分のマージン
const REWARD_IN_LAMPORTS: u64 = 10000000; // 0.001 SOL
