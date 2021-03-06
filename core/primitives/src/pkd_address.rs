#[cfg(feature = "std")]
use serde::{Serialize, Serializer, Deserialize, Deserializer};
// use primitive_types::H256;
use keys::PaymentAddress;
use fixed_hash::construct_fixed_hash;
use pairing::bls12_381::Bls12;
use crate::JUBJUB;
#[cfg(feature = "std")]
use substrate_primitives::hexdisplay::AsBytesRef;

#[cfg(feature = "std")]
use substrate_primitives::bytes;

use parity_codec::{Encode, Decode, Input};

const SIZE: usize = 32;

construct_fixed_hash! {
    pub struct H256(SIZE);
}

pub type PkdAddress = H256;

// #[derive(Eq, PartialEq, Clone, Default, Encode, Decode, Copy)]
// #[cfg_attr(feature = "std", derive(Debug, Serialize, Deserialize))]
// pub struct PkdAddress(pub H256);
// pub struct PkdAddress(pub [u8; 32]);


#[cfg(feature = "std")]
impl Serialize for PkdAddress {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> 
        where S: Serializer
    {
        bytes::serialize(&self.0, serializer)
    }
}

#[cfg(feature = "std")]
impl<'de> Deserialize<'de> for PkdAddress {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        bytes::deserialize_check_len(deserializer, bytes::ExpectedLen::Exact(SIZE))
            .map(|x| PkdAddress::from_slice(&x))
    }
}

impl Encode for PkdAddress {
    fn using_encoded<R, F: FnOnce(&[u8]) -> R>(&self, f: F) -> R {
        self.0.using_encoded(f)
    }
}

impl Decode for PkdAddress {
    fn decode<I: Input>(input: &mut I) -> Option<Self> {
        <[u8; SIZE] as Decode>::decode(input).map(H256)
    }
}

impl PkdAddress {
    pub fn into_payment_address(&self) -> Option<PaymentAddress<Bls12>> {         
        PaymentAddress::<Bls12>::read(&mut &self.0[..], &JUBJUB).ok()
    }

    pub fn from_payment_address(address: &PaymentAddress<Bls12>) -> Self {
        let mut writer = [0u8; 32];
        address.write(&mut writer[..]).unwrap();
        // PkdAddress::from_slice(&writer)      
        H256::from_slice(&writer)
    }
}

impl Into<PkdAddress> for PaymentAddress<Bls12> {
    fn into(self) -> PkdAddress {
        PkdAddress::from_payment_address(&self)
    }
}

// impl From<H256> for PkdAddress {
//     fn from(h: H256) -> Self {
//         PkdAddress(h)
//     }
// }

#[cfg(feature = "std")]
impl AsBytesRef for PkdAddress {
    fn as_bytes_ref(&self) -> &[u8] {
        self.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{Rng, SeedableRng, XorShiftRng};    
    use pairing::bls12_381::Bls12;
    use keys::*;

    #[test]
    fn test_addr_into_from() {
        let rng = &mut XorShiftRng::from_seed([0x3dbe6259, 0x8d313d76, 0x3237db17, 0xe5bc0654]);
        let mut seed = [0u8; 32];
        rng.fill_bytes(&mut seed[..]);

        let ex_sk = ExpandedSpendingKey::<Bls12>::from_spending_key(&seed[..]);
        let viewing_key = ViewingKey::<Bls12>::from_expanded_spending_key(&ex_sk, &JUBJUB);        
        let addr1 = viewing_key.into_payment_address(&JUBJUB);

        let account_id = PkdAddress::from_payment_address(&addr1);
        println!("account_id: {:?}", account_id);
        println!("pkd_address: {:?}", PkdAddress::from_slice(b"Alice                           "));
        let addr2 = account_id.into_payment_address().unwrap();
        assert!(addr1 == addr2);
    }
}
