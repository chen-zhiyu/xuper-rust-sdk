use crate::errors::Result;
use crate::{config, protos, session, wallet, xchain};

/// account在chain上面给to转账amount，小费是fee，留言是desc
pub fn invoke_contract(
    account: &wallet::Account,
    chain: &xchain::XChainClient,
    method_name: &String,
    args: std::collections::HashMap<String, Vec<u8>>,
) -> Result<String> {
    let mut invoke_req = protos::xchain::InvokeRequest::new();
    invoke_req.set_module_name(String::from("wasm"));
    invoke_req.set_contract_name(account.contract_name.to_owned());
    invoke_req.set_method_name(method_name.to_owned());
    invoke_req.set_args(args);
    invoke_req.set_amount(String::from("0"));

    let invoke_requests = vec![invoke_req; 1];
    let mut auth_requires = vec![];
    if !account.contract_account.is_empty() {
        let mut s = account.contract_account.to_owned();
        s.push_str("/");
        s.push_str(account.address.to_owned().as_str());
        auth_requires.push(s);
    };
    auth_requires.push(
        config::CONFIG
            .read()
            .unwrap()
            .compliance_check
            .compliance_check_endorse_service_addr
            .to_owned(),
    );

    let mut invoke_rpc_request = protos::xchain::InvokeRPCRequest::new();
    invoke_rpc_request.set_bcname(chain.chain_name.to_owned());
    invoke_rpc_request.set_requests(protobuf::RepeatedField::from_vec(invoke_requests));
    invoke_rpc_request.set_initiator(account.address.to_owned());
    invoke_rpc_request.set_auth_require(protobuf::RepeatedField::from_vec(auth_requires.clone()));

    let total_amount = config::CONFIG
        .read()
        .unwrap()
        .compliance_check
        .compliance_check_endorse_service_fee;

    let mut pre_sel_utxo_req = protos::xchain::PreExecWithSelectUTXORequest::new();
    pre_sel_utxo_req.set_bcname(chain.chain_name.to_owned());
    pre_sel_utxo_req.set_address(account.address.to_owned());
    pre_sel_utxo_req.set_totalAmount(total_amount as i64);
    pre_sel_utxo_req.set_request(invoke_rpc_request.clone());

    let msg = session::Message {
        to: Default::default(),
        fee: Default::default(),
        desc: String::from("call from contract"),
        auth_require: auth_requires.clone(),
        amount: Default::default(),
        frozen_height: 0,
        initiator: account.address.to_owned(),
    };

    let sess = session::Session::new(chain, account, &msg);
    let mut resp = sess.pre_exec_with_select_utxo(pre_sel_utxo_req)?;

    //TODO 代码优化
    let msg = session::Message {
        to: String::from(""),
        fee: resp.get_response().get_gas_used().to_string(),
        desc: String::from("call from contract"),
        auth_require: auth_requires,
        amount: Default::default(),
        frozen_height: 0,
        initiator: account.address.to_owned(),
    };
    let sess = session::Session::new(chain, account, &msg);
    sess.gen_complete_tx_and_post(&mut resp)
}

pub fn query_contract(
    account: &wallet::Account,
    client: &xchain::XChainClient,
    method_name: &String,
    args: std::collections::HashMap<String, Vec<u8>>,
) -> Result<protos::xchain::InvokeRPCResponse> {
    let mut invoke_req = protos::xchain::InvokeRequest::new();
    invoke_req.set_module_name(String::from("wasm"));
    invoke_req.set_contract_name(account.contract_name.to_owned());
    invoke_req.set_method_name(method_name.to_owned());
    invoke_req.set_args(args);
    let invoke_requests = vec![invoke_req; 1];
    let mut auth_requires = vec![];

    if !account.contract_account.is_empty() {
        let mut s = account.contract_account.to_owned();
        s.push_str("/");
        s.push_str(account.address.to_owned().as_str());
        auth_requires.push(s);
    };

    auth_requires.push(
        config::CONFIG
            .read()
            .unwrap()
            .compliance_check
            .compliance_check_endorse_service_addr
            .to_owned(),
    );

    let mut invoke_rpc_request = protos::xchain::InvokeRPCRequest::new();
    invoke_rpc_request.set_bcname(client.chain_name.to_owned());
    invoke_rpc_request.set_requests(protobuf::RepeatedField::from_vec(invoke_requests));
    invoke_rpc_request.set_initiator(account.address.to_owned());
    invoke_rpc_request.set_auth_require(protobuf::RepeatedField::from_vec(auth_requires.clone()));

    let msg = session::Message {
        to: String::from(""),
        fee: String::from("0"),
        desc: String::from(""),
        auth_require: auth_requires,
        amount: Default::default(),
        frozen_height: 0,
        initiator: account.address.to_owned(),
    };

    let sess = session::Session::new(client, account, &msg);
    sess.pre_exec(invoke_rpc_request)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[test]
    fn test_contract() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("key/private.key");
        let acc = super::wallet::Account::new(
            d.to_str().unwrap(),
            "counter327861",
            "XC1111111111000000@xuper",
        );
        let bcname = String::from("xuper");
        let chain = super::session::ChainClient::new(&bcname);
        let mn = String::from("increase");

        let mut args = HashMap::new();
        args.insert(String::from("key"), String::from("counter").into_bytes());

        let txid = super::invoke_contract(&acc, &chain, &mn, args);
        println!("contract txid: {:?}", txid);

        assert_eq!(txid.is_ok(), true);
        let txid = txid.unwrap();

        let msg: crate::session::Message = Default::default();
        let sess = crate::session::Session::new(&chain, &acc, &msg);
        let res = sess.query_tx(&txid);
        assert_eq!(res.is_ok(), true);
        println!("{:?}", res.unwrap());
    }

    #[test]
    fn test_query() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("key/private.key");
        let acc = super::wallet::Account::new(
            d.to_str().unwrap(),
            "counter327861",
            "XC1111111111000000@xuper",
        );
        let bcname = String::from("xuper");
        let chain = super::session::ChainClient::new(&bcname);
        let mn = String::from("get");
        let mut args = HashMap::new();
        args.insert(String::from("key"), String::from("counter").into_bytes());

        let resp = super::query_contract(&acc, &chain, &mn, args);
        assert_eq!(resp.is_ok(), true);
        println!(
            "contract query result: {}",
            std::str::from_utf8(&resp.ok().unwrap().get_response().get_response()[0]).unwrap()
        );
    }
}
