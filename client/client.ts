import * as web3 from "@solana/web3.js";
import * as anchor from "@project-serum/anchor";
import { web3 } from "@project-serum/anchor";
import { PublicKey, SystemProgram } from "@solana/web3.js";
import type { NftMarketHours } from "../target/types/nft_market_hours";

// Configure the client to use the local cluster
anchor.setProvider(anchor.AnchorProvider.env());

const program = anchor.workspace.NftMarketHours as anchor.Program<NftMarketHours>;


async function main() {
  // Solana Playgroundの接続とウォレットを取得
  const balance = await program.provider.connection.getBalance(program.provider.publicKey);
  console.log(`Playground Wallet Balance: ${balance}`);

  // プログラムIDを設定
  const programID = program.programId;

  // OracleアカウントのためのPDAを生成
  const [globalOraclePDA] = PublicKey.findProgramAddressSync(
    [Buffer.from("oracle", "utf8")],
    programID
  );

  const [rewardVaultPDA] = PublicKey.findProgramAddressSync(
    [Buffer.from("reward_vault", "utf8"), globalOraclePDA.toBuffer()],
    programID
  );

  console.log(`Oracle PDA: ${globalOraclePDA.toBase58()}`);
  console.log(`Reward Vault PDA: ${rewardVaultPDA.toBase58()}`);

  let txHash;
  let oracleDataAccount;

  try {
    // Oracleアカウントをフェッチ（初期化されているか確認）
    oracleDataAccount = await program.account.oracle.fetch(globalOraclePDA);
    console.log("Oracleアカウントは既に存在しています:", oracleDataAccount);
  } catch (err) {
    console.log("Oracleアカウントが存在しないため、作成します...");

    // Oracleアカウントを初期化するトランザクション
    txHash = await program.methods
      .createOracle()
      .accounts({
        oracle: globalOraclePDA,
        rewardVault: rewardVaultPDA,
        signer: program.provider.publicKey,
        systemProgram: web3.SystemProgram.programId,
      })
      .signers([program.provider.wallet.payer])
      .rpc();

    await logTransaction(txHash);
    console.log("Oracleアカウントの作成トランザクション:", txHash);
  }

  // OracleをCrankするトランザクション
  try {
    console.log("OracleをCrankします...");
    txHash = await program.methods
      .crankOracle()
      .accounts({
        oracle: globalOraclePDA,
        rewardVault: rewardVaultPDA,
        signer: program.provider.publicKey,
        systemProgram: web3.SystemProgram.programId,
      })
      .signers([program.provider.wallet.payer])
      .rpc();

    await logTransaction(txHash);
    console.log("Crankトランザクション:", txHash);
  } catch (err) {
    console.error("OracleをCrankする際にエラーが発生しました:", err);
  }
}

// トランザクションをログ出力する関数
// async function logTransaction(txHash: string) {
//   const txInfo = await program.provider.connection.getConfirmedTransaction(
//     txHash,
//     "finalized"
//   );
//   console.log("トランザクション情報:", txInfo);
// }

async function logTransaction(txHash) {
  const { blockhash, lastValidBlockHeight } =
    await program.provider.connection.getLatestBlockhash();

  await program.provider.connection.confirmTransaction({
    blockhash,
    lastValidBlockHeight,
    signature: txHash,
  });
}

// 実行
main().catch((err) => {
  console.error("エラー:", err);
});
