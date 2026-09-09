use {
    anchor_lang::{
        prelude::msg,
        solana_program::instruction::Instruction,
        solana_program::program_pack::Pack,
        system_program::ID as SYSTEM_PROGRAM_ID,
        InstructionData, ToAccountMetas,
    },
    anchor_spl::{
        associated_token::{self, ID as ASSOCIATED_TOKEN_PROGRAM_ID},
        token::spl_token,
    },
    litesvm::LiteSVM,
    litesvm_token::{
        spl_token::ID as TOKEN_PROGRAM_ID, CreateAssociatedTokenAccount, CreateMint, MintTo,
    },
    solana_keypair::Keypair,
    solana_message::Message,
    solana_pubkey::Pubkey,
    solana_signer::Signer,
    solana_transaction::Transaction,
    solana_clock::Clock,
};

fn setup() -> (LiteSVM, Keypair) {
    let program_id = escrowq32026::id();
    let payer = Keypair::new();
    let mut svm = LiteSVM::new();
    let bytes = include_bytes!(concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/../deploy/escrowq32026.so"
    ));
    svm.add_program(program_id, bytes).unwrap();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    (svm, payer)
}

#[test]
fn test_make_and_refund() {
    let (mut program, payer) = setup();
    let maker = payer.pubkey();

    let mint_a = CreateMint::new(&mut program, &payer)
        .decimals(6)
        .authority(&maker)
        .send()
        .unwrap();
    let mint_b = CreateMint::new(&mut program, &payer)
        .decimals(6)
        .authority(&maker)
        .send()
        .unwrap();

    let maker_ata_a = CreateAssociatedTokenAccount::new(&mut program, &payer, &mint_a)
        .owner(&maker)
        .send()
        .unwrap();

    let seed = 123u64;
    let escrow = Pubkey::find_program_address(
        &[b"escrow", maker.as_ref(), &seed.to_le_bytes()],
        &escrowq32026::id(),
    )
    .0;
    let vault = associated_token::get_associated_token_address(&escrow, &mint_a);

    MintTo::new(&mut program, &payer, &mint_a, &maker_ata_a, 1_000_000_000)
        .send()
        .unwrap();

    let make_ix = Instruction {
        program_id: escrowq32026::id(),
        accounts: escrowq32026::accounts::Make {
            maker,
            mint_a,
            mint_b,
            maker_ata_a,
            escrow,
            vault,
            associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
            token_program: TOKEN_PROGRAM_ID,
            system_program: SYSTEM_PROGRAM_ID,
        }
        .to_account_metas(None),
        data: escrowq32026::instruction::Make {
            deposit: 10_000_000,
            seed,
            receive: 10_000_000,
            expiration: 2_000_000_000,
        }
        .data(),
    };

    let message = Message::new(&[make_ix], Some(&payer.pubkey()));
    let tx = Transaction::new(&[&payer], message, program.latest_blockhash());
    program.send_transaction(tx).unwrap();

    let vault_data = spl_token::state::Account::unpack(
        &program.get_account(&vault).unwrap().data,
    )
    .unwrap();
    assert_eq!(vault_data.amount, 10_000_000);

    let refund_ix = Instruction {
        program_id: escrowq32026::id(),
        accounts: escrowq32026::accounts::Refund {
            maker,
            mint_a,
            maker_ata_a,
            escrow,
            vault,
            token_program: TOKEN_PROGRAM_ID,
            system_program: SYSTEM_PROGRAM_ID,
        }
        .to_account_metas(None),
        data: escrowq32026::instruction::Refund {}.data(),
    };

    let message = Message::new(&[refund_ix], Some(&payer.pubkey()));
    let tx = Transaction::new(&[&payer], message, program.latest_blockhash());
    program.send_transaction(tx).unwrap();

    assert!(program.get_account(&escrow).is_none());
    assert!(program.get_account(&vault).is_none());
    msg!("test_make_and_refund passed");
}

#[test]
fn test_make_update_and_take() {
    let (mut program, payer) = setup();
    let maker = payer.pubkey();
    let taker = Keypair::new();
    program.airdrop(&taker.pubkey(), 5_000_000_000).unwrap();

    let mint_a = CreateMint::new(&mut program, &payer)
        .decimals(6)
        .authority(&maker)
        .send()
        .unwrap();
    let mint_b = CreateMint::new(&mut program, &payer)
        .decimals(6)
        .authority(&maker)
        .send()
        .unwrap();

    let maker_ata_a = CreateAssociatedTokenAccount::new(&mut program, &payer, &mint_a)
        .owner(&maker)
        .send()
        .unwrap();
    let taker_ata_a = CreateAssociatedTokenAccount::new(&mut program, &taker, &mint_a)
        .owner(&taker.pubkey())
        .send()
        .unwrap();
    let taker_ata_b = CreateAssociatedTokenAccount::new(&mut program, &taker, &mint_b)
        .owner(&taker.pubkey())
        .send()
        .unwrap();

    MintTo::new(&mut program, &payer, &mint_b, &taker_ata_b, 50_000_000)
        .send()
        .unwrap();
    MintTo::new(&mut program, &payer, &mint_a, &maker_ata_a, 1_000_000_000)
        .send()
        .unwrap();

    let seed = 456u64;
    let escrow = Pubkey::find_program_address(
        &[b"escrow", maker.as_ref(), &seed.to_le_bytes()],
        &escrowq32026::id(),
    )
    .0;
    let vault = associated_token::get_associated_token_address(&escrow, &mint_a);
    let maker_ata_b = associated_token::get_associated_token_address(&maker, &mint_b);

    let make_ix = Instruction {
        program_id: escrowq32026::id(),
        accounts: escrowq32026::accounts::Make {
            maker,
            mint_a,
            mint_b,
            maker_ata_a,
            escrow,
            vault,
            associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
            token_program: TOKEN_PROGRAM_ID,
            system_program: SYSTEM_PROGRAM_ID,
        }
        .to_account_metas(None),
        data: escrowq32026::instruction::Make {
            deposit: 20_000_000,
            seed,
            receive: 15_000_000,
            expiration: 2_000_000_000,
        }
        .data(),
    };
    let message = Message::new(&[make_ix], Some(&payer.pubkey()));
    let tx = Transaction::new(&[&payer], message, program.latest_blockhash());
    program.send_transaction(tx).unwrap();

    let update_ix = Instruction {
        program_id: escrowq32026::id(),
        accounts: escrowq32026::accounts::Update {
            maker,
            escrow,
        }
        .to_account_metas(None),
        data: escrowq32026::instruction::Update {
            receive: 12_000_000,
        }
        .data(),
    };
    let message = Message::new(&[update_ix], Some(&payer.pubkey()));
    let tx = Transaction::new(&[&payer], message, program.latest_blockhash());
    program.send_transaction(tx).unwrap();

    let take_ix = Instruction {
        program_id: escrowq32026::id(),
        accounts: escrowq32026::accounts::Take {
            taker: taker.pubkey(),
            maker,
            mint_a,
            mint_b,
            taker_ata_a,
            taker_ata_b,
            maker_ata_b,
            escrow,
            vault,
            associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
            token_program: TOKEN_PROGRAM_ID,
            system_program: SYSTEM_PROGRAM_ID,
        }
        .to_account_metas(None),
        data: escrowq32026::instruction::Take {}.data(),
    };
    let message = Message::new(&[take_ix], Some(&taker.pubkey()));
    let tx = Transaction::new(&[&taker], message, program.latest_blockhash());
    program.send_transaction(tx).unwrap();

    assert!(program.get_account(&escrow).is_none());
    assert!(program.get_account(&vault).is_none());

    let taker_a = spl_token::state::Account::unpack(
        &program.get_account(&taker_ata_a).unwrap().data,
    )
    .unwrap();
    assert_eq!(taker_a.amount, 20_000_000);

    let maker_b = spl_token::state::Account::unpack(
        &program.get_account(&maker_ata_b).unwrap().data,
    )
    .unwrap();
    assert_eq!(maker_b.amount, 12_000_000);

    msg!("test_make_update_and_take passed");
}

#[test]
fn test_take_fails_when_expired() {
    let (mut program, payer) = setup();
    let maker = payer.pubkey();
    let taker = Keypair::new();
    program.airdrop(&taker.pubkey(), 5_000_000_000).unwrap();

    let mint_a = CreateMint::new(&mut program, &payer)
        .decimals(6)
        .authority(&maker)
        .send()
        .unwrap();
    let mint_b = CreateMint::new(&mut program, &payer)
        .decimals(6)
        .authority(&maker)
        .send()
        .unwrap();

    let maker_ata_a = CreateAssociatedTokenAccount::new(&mut program, &payer, &mint_a)
        .owner(&maker)
        .send()
        .unwrap();
    let taker_ata_a = CreateAssociatedTokenAccount::new(&mut program, &taker, &mint_a)
        .owner(&taker.pubkey())
        .send()
        .unwrap();
    let taker_ata_b = CreateAssociatedTokenAccount::new(&mut program, &taker, &mint_b)
        .owner(&taker.pubkey())
        .send()
        .unwrap();

    MintTo::new(&mut program, &payer, &mint_b, &taker_ata_b, 50_000_000)
        .send()
        .unwrap();
    MintTo::new(&mut program, &payer, &mint_a, &maker_ata_a, 1_000_000_000)
        .send()
        .unwrap();

    let seed = 789u64;
    let escrow = Pubkey::find_program_address(
        &[b"escrow", maker.as_ref(), &seed.to_le_bytes()],
        &escrowq32026::id(),
    )
    .0;
    let vault = associated_token::get_associated_token_address(&escrow, &mint_a);
    let maker_ata_b = associated_token::get_associated_token_address(&maker, &mint_b);

    // MAKE with expiration that we will move past
    let make_ix = Instruction {
        program_id: escrowq32026::id(),
        accounts: escrowq32026::accounts::Make {
            maker,
            mint_a,
            mint_b,
            maker_ata_a,
            escrow,
            vault,
            associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
            token_program: TOKEN_PROGRAM_ID,
            system_program: SYSTEM_PROGRAM_ID,
        }
        .to_account_metas(None),
        data: escrowq32026::instruction::Make {
            deposit: 10_000_000,
            seed,
            receive: 10_000_000,
            expiration: 50,
        }
        .data(),
    };
    let message = Message::new(&[make_ix], Some(&payer.pubkey()));
    let tx = Transaction::new(&[&payer], message, program.latest_blockhash());
    program.send_transaction(tx).unwrap();

    // Force the clock past the expiration
    let mut clock = program.get_sysvar::<Clock>();
    clock.unix_timestamp = 100;
    program.set_sysvar(&clock);

    // TAKE must fail
    let take_ix = Instruction {
        program_id: escrowq32026::id(),
        accounts: escrowq32026::accounts::Take {
            taker: taker.pubkey(),
            maker,
            mint_a,
            mint_b,
            taker_ata_a,
            taker_ata_b,
            maker_ata_b,
            escrow,
            vault,
            associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
            token_program: TOKEN_PROGRAM_ID,
            system_program: SYSTEM_PROGRAM_ID,
        }
        .to_account_metas(None),
        data: escrowq32026::instruction::Take {}.data(),
    };
    let message = Message::new(&[take_ix], Some(&taker.pubkey()));
    let tx = Transaction::new(&[&taker], message, program.latest_blockhash());
    let result = program.send_transaction(tx);
    assert!(result.is_err());
    msg!("test_take_fails_when_expired passed");
}
