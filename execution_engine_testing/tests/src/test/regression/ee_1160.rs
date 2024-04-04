use casper_engine_test_support::{
    LmdbWasmTestBuilder, TransferRequestBuilder, DEFAULT_ACCOUNT_ADDR,
    DEFAULT_ACCOUNT_INITIAL_BALANCE, LOCAL_GENESIS_REQUEST,
};
use casper_execution_engine::engine_state::WASMLESS_TRANSFER_FIXED_GAS_PRICE;
use casper_types::{account::AccountHash, Gas, MintCosts, Motes, SystemConfig, U512};

const ACCOUNT_1_ADDR: AccountHash = AccountHash::new([1u8; 32]);

#[ignore]
#[test]
fn ee_1160_wasmless_transfer_should_empty_account() {
    let wasmless_transfer_gas_cost = Gas::from(SystemConfig::default().mint_costs().transfer);
    let wasmless_transfer_cost = Motes::from_gas(
        wasmless_transfer_gas_cost,
        WASMLESS_TRANSFER_FIXED_GAS_PRICE,
    )
    .expect("gas overflow");

    let transfer_amount =
        U512::from(DEFAULT_ACCOUNT_INITIAL_BALANCE) - wasmless_transfer_cost.value();

    let mut builder = LmdbWasmTestBuilder::default();
    builder.run_genesis(LOCAL_GENESIS_REQUEST.clone());

    let default_account = builder
        .get_entity_by_account_hash(*DEFAULT_ACCOUNT_ADDR)
        .expect("should get default_account");

    let no_wasm_transfer_request_1 =
        TransferRequestBuilder::new(transfer_amount, ACCOUNT_1_ADDR).build();
    builder
        .transfer_and_commit(no_wasm_transfer_request_1)
        .expect_success();

    let last_result = builder.get_exec_result_owned(0).unwrap();

    assert!(last_result.error().is_none(), "{:?}", last_result);
    assert!(!last_result.transfers().is_empty());

    let default_account_balance_after = builder.get_purse_balance(default_account.main_purse());

    let account_1 = builder
        .get_entity_by_account_hash(ACCOUNT_1_ADDR)
        .expect("should get default_account");
    let account_1_balance = builder.get_purse_balance(account_1.main_purse());

    assert_eq!(default_account_balance_after, U512::zero());
    assert_eq!(account_1_balance, transfer_amount);
}

#[ignore]
#[test]
fn ee_1160_transfer_larger_than_balance_should_fail() {
    let transfer_amount = U512::from(DEFAULT_ACCOUNT_INITIAL_BALANCE)
        - U512::from(MintCosts::default().transfer)
        // One above the available balance to transfer should raise an InsufficientPayment already
        + U512::one();

    let mut builder = LmdbWasmTestBuilder::default();
    builder.run_genesis(LOCAL_GENESIS_REQUEST.clone());

    let default_account = builder
        .get_entity_by_account_hash(*DEFAULT_ACCOUNT_ADDR)
        .expect("should get default_account");

    let balance_before = builder.get_purse_balance(default_account.main_purse());

    let no_wasm_transfer_request_1 =
        TransferRequestBuilder::new(transfer_amount, ACCOUNT_1_ADDR).build();
    builder.transfer_and_commit(no_wasm_transfer_request_1);

    let balance_after = builder.get_purse_balance(default_account.main_purse());

    let wasmless_transfer_gas_cost = Gas::from(MintCosts::default().transfer);
    let wasmless_transfer_motes = Motes::from_gas(
        wasmless_transfer_gas_cost,
        WASMLESS_TRANSFER_FIXED_GAS_PRICE,
    )
    .expect("gas overflow");

    let last_result = builder.get_exec_result_owned(0).unwrap();
    assert_eq!(
        balance_before - wasmless_transfer_motes.value(),
        balance_after
    );
    // TODO: reenable when new payment logic is added
    //assert_eq!(last_result.cost(), wasmless_transfer_gas_cost);

    assert!(
        last_result.error().is_some(),
        "Expected error but last result is {:?}",
        last_result
    );
    assert!(
        last_result.transfers().is_empty(),
        "Expected empty list of transfers"
    );
}

#[ignore]
#[test]
fn ee_1160_large_wasmless_transfer_should_avoid_overflow() {
    let transfer_amount = U512::max_value();

    let mut builder = LmdbWasmTestBuilder::default();
    builder.run_genesis(LOCAL_GENESIS_REQUEST.clone());

    let default_account = builder
        .get_entity_by_account_hash(*DEFAULT_ACCOUNT_ADDR)
        .expect("should get default_account");

    let balance_before = builder.get_purse_balance(default_account.main_purse());

    let no_wasm_transfer_request_1 =
        TransferRequestBuilder::new(transfer_amount, ACCOUNT_1_ADDR).build();
    builder.transfer_and_commit(no_wasm_transfer_request_1);

    let balance_after = builder.get_purse_balance(default_account.main_purse());

    let wasmless_transfer_gas_cost = Gas::from(MintCosts::default().transfer);
    let wasmless_transfer_motes = Motes::from_gas(
        wasmless_transfer_gas_cost,
        WASMLESS_TRANSFER_FIXED_GAS_PRICE,
    )
    .expect("gas overflow");

    assert_eq!(
        balance_before - wasmless_transfer_motes.value(),
        balance_after
    );

    let last_result = builder.get_exec_result_owned(0).unwrap();
    // TODO: reenable when new payment logic is added
    // assert_eq!(last_result.cost(), wasmless_transfer_gas_cost);

    assert!(
        last_result.error().is_some(),
        "Expected error but last result is {:?}",
        last_result
    );
    assert!(
        last_result.transfers().is_empty(),
        "Expected empty list of transfers"
    );
}
