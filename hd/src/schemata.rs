// Descriptor wallet library extending bitcoin & miniscript functionality
// by LNP/BP Association (https://lnp-bp.org)
// Written in 2020-2022 by
//     Dr. Maxim Orlovsky <orlovsky@pandoracore.com>
//
// To the extent possible under law, the author(s) have dedicated all
// copyright and related and neighboring rights to this software to
// the public domain worldwide. This software is distributed without
// any warranty.
//
// You should have received a copy of the Apache-2.0 License
// along with this software.
// If not, see <https://opensource.org/licenses/Apache-2.0>.

//! Derivation schemata based on BIP-43-related standards.

use core::convert::TryInto;
use core::str::FromStr;
use std::convert::TryFrom;

use bitcoin::util::bip32::{ChildNumber, DerivationPath};
#[cfg(feature = "miniscript")]
use miniscript::descriptor::DescriptorType;

use crate::{HardenedIndex, UnhardenedIndex};

/// Errors in parsing derivation scheme string representation
#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug, Error, Display)]
#[display(doc_comments)]
pub enum ParseError {
    /// invalid blockchain name {0}; it must be either `bitcoin`, `testnet` or
    /// hardened index number
    InvalidBlockchainName(String),

    /// LNPBP-43 blockchain index {0} must be hardened
    UnhardenedBlockchainIndex(u32),

    /// invalid LNPBP-43 identity representation {0}
    InvalidIdentityIndex(String),

    /// invalid BIP-43 purpose {0}
    InvalidPurposeIndex(String),

    /// BIP-{0} support is not implemented (of BIP with this number does not
    /// exist)
    UnimplementedBip(u16),

    /// derivation path can't be recognized as one of BIP-43-based standards
    UnrecognizedBipScheme,

    /// BIP-43 scheme must have form of `bip43/<purpose>h`
    InvalidBip43Scheme,

    /// BIP-48 scheme must have form of `bip48-native` or `bip48-nested`
    InvalidBip48Scheme,

    /// invalid derivation path `{0}`
    InvalidDerivationPath(String),
}

/// Derivation path index specifying blockchain in LNPBP-43 format
#[derive(
    Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug, Display, From
)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(crate = "serde_crate")
)]
pub enum DerivationBlockchain {
    /// Bitcoin mainnet
    #[display("bitcoin")]
    Bitcoin,

    /// Any testnet blockchain
    #[display("testnet")]
    Testnet,

    /// Custom blockchain (non-testnet)
    #[display(inner)]
    #[from]
    Custom(HardenedIndex),
}

impl DerivationBlockchain {
    /// Returns derivation path segment child number corresponding to the given
    /// blockchain from LNPBP-43 standard
    #[inline]
    pub fn child_number(self) -> ChildNumber {
        match self {
            Self::Bitcoin => ChildNumber::Hardened { index: 0 },
            Self::Testnet => ChildNumber::Hardened { index: 1 },
            Self::Custom(index) => index.into(),
        }
    }
}

impl FromStr for DerivationBlockchain {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parsed = ChildNumber::from_str(s);
        match (s.to_lowercase().as_str(), parsed) {
            ("bitcoin", _) => Ok(Self::Bitcoin),
            ("testnet", _) => Ok(Self::Testnet),
            (_, Ok(index @ ChildNumber::Hardened { .. })) => {
                Ok(Self::Custom(index.try_into().expect(
                    "ChildNumber::Hardened failed to convert into HardenedIndex type",
                )))
            }
            (_, Ok(ChildNumber::Normal { index })) => {
                Err(ParseError::UnhardenedBlockchainIndex(index))
            }
            (wrong, Err(_)) => Err(ParseError::InvalidBlockchainName(wrong.to_owned())),
        }
    }
}

/// Specific derivation scheme after BIP-43 standards
#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug, Display)]
#[cfg_attr(feature = "clap", derive(ArgEnum))]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(crate = "serde_crate")
)]
#[non_exhaustive]
pub enum Bip43 {
    /// Account-based P2PKH derivation
    ///
    /// `m / 44' / coin_type' / account'`
    #[display("bip44", alt = "m/44h")]
    Bip44,

    /// Account-based native P2WPKH derivation
    ///
    /// `m / 84' / coin_type' / account'`
    #[display("bip84", alt = "m/84h")]
    Bip84,

    /// Account-based legacy P2WPH-in-P2SH derivation
    ///
    /// `m / 49' / coin_type' / account'`
    #[display("bip49", alt = "m/49h")]
    Bip49,

    /// Account-based single-key P2TR derivation
    ///
    /// `m / 86' / coin_type' / account'`
    #[display("bip86", alt = "m/86h")]
    Bip86,

    /// Cosigner-index-based multisig derivation
    ///
    /// `m / 45' / cosigner_index`
    #[display("bip45", alt = "m/45h")]
    Bip45,

    /// Account-based multisig derivation with sorted keys & P2WSH nested
    /// scripts
    ///
    /// `m / 48' / 1' / account' / script_type'`
    #[display("bip48-nested", alt = "m/48h//1h")]
    Bip48Nested,

    /// Account-based multisig derivation with sorted keys & P2WSH native
    /// scripts
    ///
    /// `m / 48' / 2' / account' / script_type'`
    #[display("bip48-native", alt = "m/48h//2h")]
    Bip48Native,

    /// Account- & descriptor-based derivation for multi-sig wallets
    #[display("bip87", alt = "m/87h")]
    ///
    /// `m / 87' / coin_type' / account'`
    Bip87,

    /// Generic BIP43 derivation with custom (non-standard) purpose value
    ///
    /// `m / purpose' / coin_type' / account'`
    #[display("bip43/{purpose}")]
    Bip43 {
        /// Purpose value
        purpose: HardenedIndex,
    },
}

impl FromStr for Bip43 {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.to_lowercase();
        let bip = s.strip_prefix("bip").or_else(|| s.strip_prefix("m/"));
        Ok(match bip {
            Some("44") => Bip43::Bip44,
            Some("84") => Bip43::Bip84,
            Some("49") => Bip43::Bip49,
            Some("86") => Bip43::Bip86,
            Some("45") => Bip43::Bip45,
            Some(bip48) if bip48.starts_with("48//") => match s
                .strip_prefix("bip48//")
                .and_then(|index| HardenedIndex::from_str(index).ok())
            {
                Some(script_type) if script_type == 1u8 => Bip43::Bip48Nested,
                Some(script_type) if script_type == 2u8 => Bip43::Bip48Native,
                _ => return Err(ParseError::InvalidBip48Scheme),
            },
            Some("48-nested") => Bip43::Bip48Nested,
            Some("48-native") => Bip43::Bip48Native,
            Some("87") => Bip43::Bip87,
            None if s.starts_with("bip43") => match s.strip_prefix("bip43/") {
                Some(purpose) => {
                    let purpose = HardenedIndex::from_str(purpose)
                        .map_err(|_| ParseError::InvalidPurposeIndex(purpose.to_owned()))?;
                    Bip43::Bip43 { purpose }
                }
                None => return Err(ParseError::InvalidBip43Scheme),
            },
            Some(_) | None => return Err(ParseError::UnrecognizedBipScheme),
        })
    }
}

/// Methods for derivation standard enumeration types.
pub trait DerivationStandard {
    /// Reconstructs derivation scheme used by the provided derivation path, if
    /// possible.
    fn with(derivation: &DerivationPath) -> Option<Self>
    where
        Self: Sized;

    /// Get hardened index matching BIP-43 purpose value, if any.
    fn purpose(&self) -> Option<HardenedIndex>;

    /// Construct derivation path for the account origin.
    fn to_origin_derivation(&self, blockchain: DerivationBlockchain) -> DerivationPath;

    /// Construct derivation path up to the provided account index segment.
    fn to_account_derivation(
        &self,
        account_index: ChildNumber,
        blockchain: DerivationBlockchain,
    ) -> DerivationPath;

    /// Construct full derivation path including address index and case
    /// (main, change etc).
    fn to_key_derivation(
        &self,
        account_index: ChildNumber,
        blockchain: DerivationBlockchain,
        index: UnhardenedIndex,
        case: Option<UnhardenedIndex>,
    ) -> DerivationPath;

    /// Check whether provided descriptor type can be used with this derivation
    /// scheme.
    fn check_descriptor_type(&self, descriptor_type: DescriptorType) -> bool;
}

impl DerivationStandard for Bip43 {
    fn with(derivation: &DerivationPath) -> Option<Bip43> {
        let mut iter = derivation.into_iter();
        let first = iter
            .next()
            .copied()
            .map(HardenedIndex::try_from)
            .transpose()
            .ok()??;
        let fourth = iter.nth(3).copied().map(HardenedIndex::try_from);
        Some(match (first, fourth) {
            (HardenedIndex(44), ..) => Bip43::Bip44,
            (HardenedIndex(84), ..) => Bip43::Bip84,
            (HardenedIndex(49), ..) => Bip43::Bip49,
            (HardenedIndex(86), ..) => Bip43::Bip86,
            (HardenedIndex(45), ..) => Bip43::Bip45,
            (HardenedIndex(87), ..) => Bip43::Bip87,
            (HardenedIndex(48), Some(Ok(script_type))) if script_type == 1u8 => Bip43::Bip48Nested,
            (HardenedIndex(48), Some(Ok(script_type))) if script_type == 2u8 => Bip43::Bip48Native,
            (HardenedIndex(48), _) => return None,
            (purpose, ..) => Bip43::Bip43 { purpose },
        })
    }

    fn purpose(&self) -> Option<HardenedIndex> {
        Some(match self {
            Bip43::Bip44 => HardenedIndex(44),
            Bip43::Bip84 => HardenedIndex(84),
            Bip43::Bip49 => HardenedIndex(49),
            Bip43::Bip86 => HardenedIndex(86),
            Bip43::Bip45 => HardenedIndex(45),
            Bip43::Bip48Nested | Bip43::Bip48Native => HardenedIndex(48),
            Bip43::Bip87 => HardenedIndex(87),
            Bip43::Bip43 { purpose } => *purpose,
        })
    }

    fn to_origin_derivation(&self, blockchain: DerivationBlockchain) -> DerivationPath {
        let mut path = Vec::with_capacity(2);
        if let Some(purpose) = self.purpose() {
            path.push(purpose.into())
        }
        path.push(blockchain.child_number());
        path.into()
    }

    fn to_account_derivation(
        &self,
        account_index: ChildNumber,
        blockchain: DerivationBlockchain,
    ) -> DerivationPath {
        let mut path = Vec::with_capacity(2);
        path.push(account_index);
        if self == &Bip43::Bip48Native {
            path.push(HardenedIndex::from(2u8).into());
        } else if self == &Bip43::Bip48Nested {
            path.push(HardenedIndex::from(1u8).into());
        }
        let derivation = self.to_origin_derivation(blockchain);
        derivation.extend(&path);
        derivation
    }

    fn to_key_derivation(
        &self,
        account_index: ChildNumber,
        blockchain: DerivationBlockchain,
        index: UnhardenedIndex,
        case: Option<UnhardenedIndex>,
    ) -> DerivationPath {
        let mut derivation = self.to_account_derivation(account_index, blockchain);
        derivation = derivation.extend(&[index.into()]);
        derivation = case
            .map(|case| derivation.extend(&[case.into()]))
            .unwrap_or(derivation);
        derivation
    }

    fn check_descriptor_type(&self, descriptor_type: DescriptorType) -> bool {
        match (self, descriptor_type) {
            (Bip43::Bip44, DescriptorType::Pkh)
            | (Bip43::Bip84, DescriptorType::Wpkh)
            | (Bip43::Bip49, DescriptorType::ShWpkh)
            | (Bip43::Bip86, DescriptorType::Tr)
            | (Bip43::Bip45, DescriptorType::ShSortedMulti)
            | (Bip43::Bip87, DescriptorType::ShSortedMulti)
            | (Bip43::Bip87, DescriptorType::ShWshSortedMulti)
            | (Bip43::Bip87, DescriptorType::WshSortedMulti) => true,
            (Bip43::Bip48Nested, DescriptorType::ShWshSortedMulti) => true,
            (Bip43::Bip48Native, DescriptorType::WshSortedMulti) => true,
            (_, _) => false,
        }
    }
}

#[cfg(not(feature = "miniscript"))]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum DescriptorType {
    /// Bare descriptor(Contains the native P2pk)
    Bare,
    /// Pure Sh Descriptor. Does not contain nested Wsh/Wpkh
    Sh,
    /// Pkh Descriptor
    Pkh,
    /// Wpkh Descriptor
    Wpkh,
    /// Wsh
    Wsh,
    /// Sh Wrapped Wsh
    ShWsh,
    /// Sh wrapped Wpkh
    ShWpkh,
    /// Sh Sorted Multi
    ShSortedMulti,
    /// Wsh Sorted Multi
    WshSortedMulti,
    /// Sh Wsh Sorted Multi
    ShWshSortedMulti,
    /// Tr Descriptor
    Tr,
}
