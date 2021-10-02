// Copyright 2017 rust-ipfs-api Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
//

use ipfs_api_examples::ipfs_api::{IpfsApi, IpfsClient};
use std::fs::File;
use std::env;
use anyhow::Result;
use ethers::{
    prelude::*,
    utils::{compile_and_launch_ganache, Ganache, Solc},
};
use std::{convert::TryFrom, sync::Arc, time::Duration};
abigen!(
    SimpleContract,
    r#"[
        function setValue(string)
        function getValue() external view returns (string)
        event ValueChanged(address indexed author, string oldValue, string newValue)
    ]"#,
    event_derives(serde::Deserialize, serde::Serialize)
);
// Creates an Ipfs client, and adds this source file to Ipfs.
//
#[ipfs_api_examples::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    println!("{:?}", &args[1]);
    
    tracing_subscriber::fmt::init();

    eprintln!("note: this must be run in the root of the project repository");
    eprintln!("connecting to localhost:5001...");

    let client = IpfsClient::default();
    let file = File::open(&args[1]).expect("could not read source file");

    match client.add(file).await {
        Ok(file) => {
            let cid = file.hash;
            eprintln!("added file: {:?}", cid);
            // 1. compile the contract (note this requires that you are inside the `examples` directory) and launch ganache
            let (compiled, ganache) = compile_and_launch_ganache(Solc::new("**/contract.sol"), Ganache::new()).await?;
            let contract = compiled.get("SimpleStorage").expect("could not find contract");
            // 2. instantiate our wallet
            let wallet: LocalWallet = ganache.keys()[0].clone().into();

            // 3. connect to the network
            let provider = Provider::<Http>::try_from(ganache.endpoint())?.interval(Duration::from_millis(10u64));

            // 4. instantiate the client with the wallet
            let client = SignerMiddleware::new(provider, wallet);
            let client = Arc::new(client);

            // 5. create a factory which will be used to deploy instances of the contract
            let factory = ContractFactory::new(
                contract.abi.clone(),
                contract.bytecode.clone(),
                client.clone(),
                );

            // 6. deploy it with the constructor arguments
            let contract = factory
                .deploy("initial value".to_string())?
                .legacy()
                .send()
                .await?;

            // 7. get the contract's address
            let addr = contract.address();

            // 8. instantiate the contract
            let contract = SimpleContract::new(addr, client.clone());

            // 9. call the `setValue` method
            // (first `await` returns a PendingTransaction, second one waits for it to be mined)
    
            let _receipt = contract
                .set_value(cid.to_owned())
                .legacy()
                .send()
                .await?
                .await?;

            // 10. get all events
            let logs = contract
                .value_changed_filter()
                .from_block(0u64)
                .query()
                .await?;

            // 11. get the new value
            let value = contract.get_value().call().await?;

            println!("Value: {}. Logs: {}", value, serde_json::to_string(&logs)?);
           
        }
        Err(e) => eprintln!("error adding file: {}", e),
    }
    Ok(())
}
