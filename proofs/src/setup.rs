use pairing::bls12_381::Bls12;

use bellman::{
    Circuit,
    ConstraintSystem,
    SynthesisError,
};

use bellman::groth16::{
    Proof,
    generate_random_parameters,
    prepare_verifying_key,
    create_random_proof,
    verify_proof,
};

use bellman_verifier::PreparedVerifyingKey;

use scrypto::jubjub::{JubjubBls12};

use rand::{OsRng, Rand};

use crate::circuit_transfer::Transfer;

pub fn setup() {
    let rng = &mut OsRng::new().expect("should be able to construct RNG");

    let params = JubjubBls12::new();

    // Create parameters for the confidential transfer circuit
    let params = {
        let c = Transfer::<Bls12> {
            params: &params,
            value: None,
            remaining_balance: None,
            randomness: None,
            alpha: None,
            proof_generation_key: None,
            ivk: None,
            pk_d_recipient: None,
            encrypted_balance: None,
        };

        generate_random_parameters(c, rng).unwrap()
    };

    let pvk = prepare_verifying_key(&params.vk);
    let mut v = vec![];
    pvk.write(&mut &mut v).unwrap();
    println!("pvk: {:?}", v);
    println!("pvk: {:?}", v.len());

    // let pvk2 = PreparedVerifyingKey::read(&mut &v)?;

    // assert!(pvk == pvk2);
}

#[test]
fn test_setup() {    
    setup()
}