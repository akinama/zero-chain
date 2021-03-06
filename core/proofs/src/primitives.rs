use pairing::{
    PrimeField,
    PrimeFieldRepr,       
};    

use scrypto::{
        jubjub::{
            JubjubEngine,
            JubjubParams,
            edwards,
            PrimeOrder,
            FixedGenerators,
            ToUniform,
        }        
};
use std::io;

use blake2_rfc::{
    blake2s::Blake2s, 
    blake2b::{Blake2b, Blake2bResult}
};

pub const PRF_EXPAND_PERSONALIZATION: &'static [u8; 16] = b"zech_ExpandSeed_";
pub const CRH_IVK_PERSONALIZATION: &'static [u8; 8] = b"zech_ivk";
pub const KEY_DIVERSIFICATION_PERSONALIZATION: &'static [u8; 8] = b"zech_div";

fn prf_expand(sk: &[u8], t: &[u8]) -> Blake2bResult {
    prf_expand_vec(sk, &vec![t])
}

fn prf_expand_vec(sk: &[u8], ts: &[&[u8]]) -> Blake2bResult {
    let mut h = Blake2b::with_params(64, &[], &[], PRF_EXPAND_PERSONALIZATION);
    h.update(sk);
    for t in ts {
        h.update(t);
    }
    h.finalize()
}

/// Extend the secret key to 64 bits for the scalar field generation.
pub fn prf_extend_wo_t(sk: &[u8]) -> Blake2bResult {
    let mut h = Blake2b::with_params(64, &[], &[], PRF_EXPAND_PERSONALIZATION);
    h.update(sk);
    h.finalize()
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExpandedSpendingKey<E: JubjubEngine> {
    pub ask: E::Fs,
    pub nsk: E::Fs,
}

impl<E: JubjubEngine> ExpandedSpendingKey<E> {
    /// Generate the 64bytes extend_spending_key from the 32bytes spending key.
    pub fn from_spending_key(sk: &[u8]) -> Self {
        let ask = E::Fs::to_uniform(prf_expand(sk, &[0x00]).as_bytes());
        let nsk = E::Fs::to_uniform(prf_expand(sk, &[0x01]).as_bytes());
        ExpandedSpendingKey { ask, nsk }
    }

    pub fn into_proof_generation_key(&self, params: &E::Params) -> ProofGenerationKey<E> {
        ProofGenerationKey {
            ak: params.generator(FixedGenerators::NoteCommitmentRandomness).mul(self.ask, params),
            nsk: self.nsk,
        }
    }

    pub fn write<W: io::Write>(&self, mut writer: W) -> io::Result<()> {
        self.ask.into_repr().write_le(&mut writer)?;
        self.nsk.into_repr().write_le(&mut writer)?;
        Ok(())
    }

    pub fn read<R: io::Read>(mut reader: R) -> io::Result<Self> {
        let mut ask_repr = <E::Fs as PrimeField>::Repr::default();
        ask_repr.read_le(&mut reader)?;
        let ask = E::Fs::from_repr(ask_repr)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        let mut nsk_repr = <E::Fs as PrimeField>::Repr::default();
        nsk_repr.read_le(&mut reader)?;
        let nsk = E::Fs::from_repr(nsk_repr)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        Ok(ExpandedSpendingKey {
            ask,
            nsk,
        })
    }
} 

#[derive(Clone)]
pub struct ProofGenerationKey<E: JubjubEngine> {
    pub ak: edwards::Point<E, PrimeOrder>,
    pub nsk: E::Fs
}

impl<E: JubjubEngine> ProofGenerationKey<E> {
    /// Generate viewing key from proof generation key.
    pub fn into_viewing_key(&self, params: &E::Params) -> ViewingKey<E> {
        ViewingKey {
            ak: self.ak.clone(),
            nk: params.generator(FixedGenerators::NoteCommitmentRandomness).mul(self.nsk, params)
        }
    }    
}

#[derive(Clone)]
pub struct ViewingKey<E: JubjubEngine> {
    pub ak: edwards::Point<E, PrimeOrder>,
    pub nk: edwards::Point<E, PrimeOrder>
}

impl<E: JubjubEngine> ViewingKey<E> {
    /// Generate viewing key from extended spending key
    pub fn from_expanded_spending_key(
        expsk: &ExpandedSpendingKey<E>, 
        params: &E::Params
    ) -> Self 
    {
        ViewingKey {
            ak: params
                .generator(FixedGenerators::NoteCommitmentRandomness)
                .mul(expsk.ask, params),
            nk: params
                .generator(FixedGenerators::NoteCommitmentRandomness)
                .mul(expsk.nsk, params),
        }
    }

    /// Generate the signature verifying key
    pub fn rk(
        &self,
        ar: E::Fs,
        params: &E::Params
    ) -> edwards::Point<E, PrimeOrder> {
        self.ak.add(
            &params.generator(FixedGenerators::NoteCommitmentRandomness).mul(ar, params),
            params
        )
    }

    /// Generate the internal viewing key
    pub fn ivk(&self) -> E::Fs {
        let mut preimage = [0; 64];
        self.ak.write(&mut &mut preimage[0..32]).unwrap();
        self.nk.write(&mut &mut preimage[32..64]).unwrap();

        let mut h = Blake2s::with_params(32, &[], &[], CRH_IVK_PERSONALIZATION);
        h.update(&preimage);
        let mut h = h.finalize().as_ref().to_vec();

        h[31] &= 0b0000_0111;
        let mut e = <E::Fs as PrimeField>::Repr::default();

        // Reads a little endian integer into this representation.
        e.read_le(&mut &h[..]).unwrap();
        E::Fs::from_repr(e).expect("should be a vaild scalar")
    }

    /// Generate the payment address from viewing key.
    pub fn into_payment_address(
        &self,        
        params: &E::Params
    ) -> PaymentAddress<E>
    {
        let pk_d = params
            .generator(FixedGenerators::NoteCommitmentRandomness)
            .mul(self.ivk(), params);

        PaymentAddress(pk_d)
    }
}

#[derive(Clone, PartialEq)]
pub struct PaymentAddress<E: JubjubEngine> (
    pub edwards::Point<E, PrimeOrder>    
);

impl<E: JubjubEngine> PaymentAddress<E> {    
    pub fn write<W: io::Write>(&self, mut writer: W) -> io::Result<()> {
        self.0.write(&mut writer)?;        
        Ok(())
    }

    pub fn read<R: io::Read>(reader: &mut R, params: &E::Params) -> io::Result<Self> {
        let pk_d = edwards::Point::<E, _>::read(reader, params)?;
        let pk_d = pk_d.as_prime_order(params).unwrap();        
        Ok(PaymentAddress(pk_d))
    }
}
