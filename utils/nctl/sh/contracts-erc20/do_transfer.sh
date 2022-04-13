#!/usr/bin/env bash

source "$NCTL"/sh/utils/main.sh
source "$NCTL"/sh/contracts-erc20/utils.sh

#######################################
# Transfers ERC-20 between test user accounts.
# Arguments:
#   Amount of ERC-20 token to approve.
#######################################
function main()
{
    local AMOUNT=${1}
    local CHAIN_NAME
    local DEPLOY_HASH
    local GAS_PAYMENT
    local NODE_ADDRESS
    local PATH_TO_CLIENT
    local USER_1_ACCOUNT_HASH
    local USER_1_ACCOUNT_KEY
    local USER_1_ID
    local USER_2_ACCOUNT_HASH
    local USER_2_ACCOUNT_KEY
    local USER_2_ID

    # Set standard deploy parameters.
    CHAIN_NAME=$(get_chain_name)
    GAS_PAYMENT=${GAS_PAYMENT:-$NCTL_DEFAULT_GAS_PAYMENT}
    NODE_ADDRESS=$(get_node_address_rpc)
    PATH_TO_CLIENT=$(get_path_to_client)

    # Set contract owner account key - i.e. faucet account.
    CONTRACT_OWNER_ACCOUNT_KEY=$(get_account_key "$NCTL_ACCOUNT_TYPE_FAUCET")

    # Set contract owner secret key.
    CONTRACT_OWNER_SECRET_KEY=$(get_path_to_secret_key "$NCTL_ACCOUNT_TYPE_FAUCET")

    # Set contract hash (hits node api).
    CONTRACT_HASH=$(get_erc20_contract_hash "$CONTRACT_OWNER_ACCOUNT_KEY")

    # Enumerate set of users.
    for USER_1_ID in $(seq 1 "$(get_count_of_users)")
    do
        # Set user 2 id. 
        if [ "$USER_1_ID" -lt "$(get_count_of_users)" ]; then
            USER_2_ID=$((USER_1_ID + 1))
        else
            USER_2_ID=1
        fi

        # Set user account info. 
        USER_1_ACCOUNT_KEY=$(get_account_key "$NCTL_ACCOUNT_TYPE_USER" "$USER_1_ID")
        USER_2_ACCOUNT_KEY=$(get_account_key "$NCTL_ACCOUNT_TYPE_USER" "$USER_2_ID")
        USER_1_ACCOUNT_HASH=$(get_account_hash "$USER_1_ACCOUNT_KEY")
        USER_2_ACCOUNT_HASH=$(get_account_hash "$USER_2_ACCOUNT_KEY")

        # Dispatch deploy (hits node api). 
        DEPLOY_HASH=$(
            $PATH_TO_CLIENT put-deploy \
                --chain-name "$CHAIN_NAME" \
                --node-address "$NODE_ADDRESS" \
                --payment-amount "$GAS_PAYMENT" \
                --secret-key "$CONTRACT_OWNER_SECRET_KEY" \
                --ttl "1day" \
                --session-hash "$CONTRACT_HASH" \
                --session-entry-point "transferFrom" \
                --session-arg "$(get_cl_arg_account_hash 'owner' "$USER_1_ACCOUNT_HASH")" \
                --session-arg "$(get_cl_arg_account_hash 'recipient' "$USER_2_ACCOUNT_HASH")" \
                --session-arg "$(get_cl_arg_u256 'amount' "$AMOUNT")" \
                | jq '.result.deploy_hash' \
                | sed -e 's/^"//' -e 's/"$//'
            )
        log "token transfer from user $USER_1_ID -> $USER_2_ID deploy hash = $DEPLOY_HASH"
    done
}

# ----------------------------------------------------------------
# ENTRY POINT
# ----------------------------------------------------------------

unset AMOUNT

for ARGUMENT in "$@"
do
    KEY=$(echo "$ARGUMENT" | cut -f1 -d=)
    VALUE=$(echo "$ARGUMENT" | cut -f2 -d=)
    case "$KEY" in
        amount) AMOUNT=${VALUE} ;;
        *)
    esac
done

main "${AMOUNT:-1000000000}"
