#[cfg(feature = "std")]
use serde::{Serialize, Serializer, Deserialize, Deserializer};
use crate::keys::PaymentAddress;
use fixed_hash::construct_fixed_hash;
use pairing::bls12_381::Bls12;
use crate::JUBJUB;

#[cfg(feature = "std")]
use substrate_primitives::bytes;

const SIZE: usize = 32;

construct_fixed_hash! {
    pub struct H256(SIZE);
}

pub type AccountId = H256;

#[cfg(feature = "std")]
impl Serialize for AccountId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> 
        where S: Serializer
    {
        bytes::serialize(&self.0, serializer)
    }
}

#[cfg(feature = "std")]
impl<'de> Deserialize<'de> for AccountId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        bytes::deserialize_check_len(deserializer, bytes::ExpectedLen::Exact(SIZE))
            .map(|x| AccountId::from_slice(&x))
    }
}

impl codec::Encode for AccountId {
    fn using_encoded<R, F: FnOnce(&[u8]) -> R>(&self, f: F) -> R {
        self.0.using_encoded(f)
    }
}

impl codec::Decode for AccountId {
    fn decode<I: codec::Input>(input: &mut I) -> Option<Self> {
        <[u8; SIZE] as codec::Decode>::decode(input).map(H256)
    }
}

impl AccountId {
    pub fn into_payment_address(&self) -> Option<PaymentAddress<Bls12>> {         
        PaymentAddress::<Bls12>::read(&mut &self.0[..], &JUBJUB).ok()
    }

    pub fn from_payment_address(address: &PaymentAddress<Bls12>) -> Self {
        let mut writer = [0u8; 32];
        address.write(&mut writer[..]).unwrap();
        AccountId::from_slice(&writer)      
    }
}

impl Into<AccountId> for PaymentAddress<Bls12> {
    fn into(self) -> AccountId {
        AccountId::from_payment_address(&self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{Rng, SeedableRng, XorShiftRng};    
    use pairing::bls12_381::Bls12;
    use crate::keys::*;

    #[test]
    fn test_addr_into_from() {
        let rng = &mut XorShiftRng::from_seed([0x3dbe6259, 0x8d313d76, 0x3237db17, 0xe5bc0654]);
        let mut seed = [0u8; 32];
        rng.fill_bytes(&mut seed[..]);

        let ex_sk = ExpandedSpendingKey::<Bls12>::from_spending_key(&seed[..]);
        let viewing_key = ViewingKey::<Bls12>::from_expanded_spending_key(&ex_sk, &JUBJUB);        
        let addr1 = viewing_key.into_payment_address(&JUBJUB);

        let account_id = AccountId::from_payment_address(&addr1);
        println!("account_id: {:?}", account_id);
        let addr2 = account_id.into_payment_address().unwrap();
        assert!(addr1 == addr2);
    }
}