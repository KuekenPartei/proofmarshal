use std::cell::Cell;
use std::convert::TryFrom;
use std::marker::PhantomData;
use std::error;
use std::borrow::{Borrow, BorrowMut};
use std::mem::{self, ManuallyDrop, MaybeUninit};
use std::ops::DerefMut;
use std::ptr;

use thiserror::Error;

use hoard::blob::{Blob, BlobDyn, Bytes, BytesUninit};
use hoard::bag::Bag;
use hoard::primitive::Primitive;
use hoard::owned::{IntoOwned, Take, Ref, Own};
use hoard::pointee::Pointee;
use hoard::zone::{Alloc, Get, GetMut, Ptr, PtrBlob, Zone};
use hoard::load::{Load, LoadRef, MaybeValid};

use crate::commit::Digest;
use crate::collections::perfecttree::height::*;
use crate::collections::perfecttree::{SumPerfectTree, SumPerfectTreeDyn, JoinError};
use crate::collections::merklesum::MerkleSum;

pub mod length;
use self::length::*;

pub mod peaktree;

/*
#[derive(Debug)]
pub struct SumMMR<T, S: Copy, Z, P: Ptr = <Z as Zone>::Ptr, L: ?Sized + ToLength = Length> {
    marker: PhantomData<T>,
    zone: Z,
    tip_ptr: MaybeUninit<P>,
    tip_digest: Cell<Option<Digest>>,
    sum: Cell<Option<S>>,
    len: L,
}

pub type MMR<T, Z, P = <Z as Zone>::Ptr> = SumMMR<T, (), Z, P>;

/*
*/
*/

/*
pub struct SumMMR<T, S: Copy, Z, P: Ptr = <Z as Zone>::Ptr, L: ?Sized + ToLength = Length> {
    state: State<T, S, Z, P>,
    len: L,
}

union State<T, S: Copy, Z, P: Ptr = <Z as Zone>::Ptr> {
    empty: ManuallyDrop<Z>,
    peak: ManuallyDrop<SumPerfectTree<T, S, Z, P, DummyHeight>>,
    peaks: ManuallyDrop<PeakTree<T, S, Z, P, DummyInnerLength>>,
}
*/

/*
pub struct Inner<T, S: Copy, Z, P: Ptr = <Z as Zone>::Ptr, L: ?Sized + ToInnerLength = InnerLength> {
    left: SumMMR<T, S, Z, P, DummyLength>,
    right: SumMMR<T, S, Z, P, DummyLength>,
    len: L,
}

pub type InnerDyn<T, S, Z, P = <Z as Zone>::Ptr> = Inner<T, S, Z, P, InnerLengthDyn>;
*/



/*
union SumMMRDynUnion<T, S: Copy, Z, P: Ptr> {
    empty: (),
    peak: ManuallyDrop<SumPerfectTree<T, S, Z, P, DummyHeight>>,
    peaks: ManuallyDrop<Peaks<T, S, Z, P, DummyInnerLength>>,
}

pub struct SumMMRDyn<T, S: Copy, Z, P: Ptr = <Z as Zone>::Ptr> {
    payload: SumMMRDynUnion<T, S, Z, P>,
    len: LengthDyn,
}
*/

/*
pub enum Tip<Peak, Inner> {
    Empty,
    Peak(Peak),
    Inner(Inner),
}

impl<T, S: Copy, Z, P: Ptr> SumMMR<T, S, Z, P> {
    pub fn new_in(zone: Z) -> Self
        where S: Default
    {
        unsafe {
            Self::from_raw_parts(
                zone,
                None,
                Some(Digest::default()),
                Some(S::default()),
                0.into(),
            )
        }
    }

    pub fn from_value_in(value: T, zone: impl BorrowMut<Z>) -> Self
        where Z: Alloc<Ptr = P>,
    {
        Self::from_peak(SumPerfectTree::new_leaf_in(value, zone))
    }

    pub fn from_peak(peak: SumPerfectTree<T, S, Z, P>) -> Self {
        let (tip_digest, sum, zone, ptr, height) = peak.into_raw_parts();
        unsafe {
            Self::from_raw_parts(
                zone,
                Some(ptr),
                tip_digest,
                sum,
                Length::from_height(height),
            )
        }
    }

    pub fn from_inner_in(inner: Inner<T, S, Z, P>, mut zone: impl BorrowMut<Z>) -> Self
        where Z: Alloc<Ptr = P>
    {
        let inner_bag: Bag<InnerDyn<T, S, Z, P>, Z, P> = zone.borrow_mut().alloc(inner);
        let (tip_ptr, len, zone) = inner_bag.into_raw_parts();

        unsafe {
            Self::from_raw_parts(
                zone,
                Some(tip_ptr),
                Some(Digest::default()),
                None,
                len.into(),
            )
        }
    }
}

#[derive(Debug)]
pub enum PushError<T, ZoneError> {
    LengthOverflow(T),
    Zone(ZoneError),
}

impl<T, Z> From<Z> for PushError<T, Z> {
    fn from(err: Z) -> Self {
        PushError::Zone(err)
    }
}

impl<T, S: MerkleSum<T>, Z: Zone> SumMMR<T, S, Z>
where T: Load,
      S: Blob + Default
{
    pub fn try_push(&mut self, value: T) -> Result<(), PushError<T, Z::Error>>
        where Z: Alloc + GetMut
    {
        if self.len() == usize::MAX {
            Err(PushError::LengthOverflow(value))
        } else {
            let peak = SumPerfectTree::new_leaf_in(value, self.zone);
            match self.try_push_peak(peak) {
                Ok(()) => Ok(()),
                Err(PushError::Zone(z)) => Err(PushError::Zone(z)),
                Err(PushError::LengthOverflow(_)) => {
                    unreachable!("overflow already checked")
                },
            }
        }
    }

    pub fn try_push_peak(&mut self, peak: SumPerfectTree<T, S, Z>)
        -> Result<(), PushError<SumPerfectTree<T, S, Z>, Z::Error>>
        where Z: Alloc + GetMut
    {
        match self.len.checked_add(peak.len()) {
            None => Err(PushError::LengthOverflow(peak)),
            Some(_new_len) if self.len() == 0 => {
                *self = Self::from_peak(peak);
                Ok(())
            },
            Some(_new_len) if self.len() == peak.len() => {
                if let Tip::Peak(old_peak) = self.take_tip()? {
                    todo!("same height")
                } else {
                    unreachable!()
                }
            },
            Some(new_len) => {
                todo!()
            },
        }

        /*
        if let Some(new_len) = self.len.checked_add(peak.len()) {
            let new_len = NonZeroLength::try_from(new_len).unwrap();

            if let Ok(old_len) = NonZeroLength::try_from(self.len()) {
                assert!(old_len.min_height() >= peak.height());

                // There already are peaks in this MMR, so we need to join those peaks to either
                // form an Inner node or a perfect tree.
                //
                // 

                /*
                if let Ok(old_len) = InnerLength::try_from(old_len) {
                    if let Tip::Inner(old_inner) = self.take_tip()? {
                        todo!("old_len = 0b{:b} new_len = 0b{:b}", old_len, new_len)
                    } else {
                        unreachable!()
                    }
                } else if let Tip::Peak(existing_peak) = self.take_tip()? {
                    match Inner::try_join(existing_peak, peak) {
                        Ok(inner) => {
                            *self = Self::from_inner_in(inner, self.zone);
                        },
                        Err(InnerJoinError::Peak(peak)) => {
                            *self = Self::from_peak(peak);
                        },
                        Err(InnerJoinError::HeightOverflow { .. }) => {
                            unreachable!("overflow already checked")
                        }
                    }
                */
                } else {
                    unreachable!("we should have a peak")
                }
            } else {
                *self = Self::from_peak(peak);
            }
            Ok(())
        } else {
            Err(PushError::LengthOverflow(peak))
        }
            */
    }

    /// Takes the MMR tip, setting the length to zero.
    pub fn take_tip(&mut self) -> Result<Tip<SumPerfectTree<T, S, Z>, Inner<T, S, Z>>, Z::Error>
        where Z: Get
    {
        match self.len() {
            0 => Ok(Tip::Empty),
            1 => {
                let this = mem::replace(self, Self::new_in(self.zone));
                let (zone, tip_ptr, tip_digest, sum, _len) = this.into_raw_parts();
                todo!()
            },
            len if len.is_power_of_two() => {
                todo!()
            },
            len => {
                todo!()

            }
        }

        /*
        if let Ok(len) = NonZeroLength::try_from(self.len()) {
            /*
            self.sum.set(Some(S::default()));
            self.tip_digest.set(None);
            self.len = Length(0);
            let tip_ptr = unsafe { self.tip_ptr.as_ptr().read() };

            match len.try_into_inner_length() {
                Ok(len) => {
                    let inner = unsafe {
                        self.zone.take_unchecked::<InnerDyn<T, S, Z>>(tip_ptr, len)?
                    };
                    Ok(Tip::Inner(inner.trust()))
                },
                Err(height) => {
                    let peak = unsafe {
                        self.zone.take_unchecked::<SumPerfectTreeDyn<T, S, Z>>(tip_ptr, height)?
                    };
                    Ok(Tip::Peak(peak.trust()))
                },
            }
            */ todo!()
        } else {
            Ok(Tip::Empty)
        }
        */
    }
}

impl<T, S: Copy, Z, P: Ptr> Default for SumMMR<T, S, Z, P>
where S: Default,
      Z: Default
{
    fn default() -> Self {
        Self::new_in(Z::default())
    }
}

impl<T, S: Copy, Z, P: Ptr, L: ToLength> SumMMR<T, S, Z, P, L> {
    pub unsafe fn from_raw_parts(
        zone: Z,
        tip_ptr: Option<P>,
        tip_digest: Option<Digest>,
        sum: Option<S>,
        len: L,
    ) -> Self {
        Self {
            marker: PhantomData,
            zone,
            tip_ptr: tip_ptr.map(MaybeUninit::new).unwrap_or(MaybeUninit::uninit()),
            tip_digest: tip_digest.into(),
            sum: sum.into(),
            len,
        }
    }

    pub fn into_raw_parts(self) -> (Z, Option<P>, Option<Digest>, Option<S>, L) {
        let this = ManuallyDrop::new(self);
        unsafe {
            (ptr::read(&this.zone),
             this.tip_ptr().map(|tip_ptr| ptr::read(tip_ptr)),
             this.tip_digest.get(),
             this.sum.get(),
             ptr::read(&this.len))
        }
    }

    fn strip(self) -> SumMMR<T, S, Z, P, DummyLength> {
        let (zone, tip_ptr, tip_digest, sum, _len) = self.into_raw_parts();
        unsafe {
            SumMMR::from_raw_parts(zone, tip_ptr, tip_digest, sum, DummyLength)
        }
    }
}

impl<T, S: Copy, Z, P: Ptr, L: ?Sized + ToLength> SumMMR<T, S, Z, P, L> {
    pub fn len(&self) -> usize {
        self.len.to_length().into()
    }

    fn tip_ptr(&self) -> Option<&P> {
        if self.len() > 0 {
            unsafe {
                Some(&*self.tip_ptr.as_ptr())
            }
        } else {
            None
        }
    }

    fn tip_ptr_mut(&mut self) -> Option<&mut P> {
        if self.len() > 0 {
            unsafe {
                Some(&mut *self.tip_ptr.as_mut_ptr())
            }
        } else {
            None
        }
    }

    pub fn try_get_dirty_tip(&self) -> Result<Tip<&SumPerfectTreeDyn<T, S, Z, P>,
                                                  &InnerDyn<T, S, Z, P>>,
                                              P::Clean>
    {
        if let Ok(len) = NonZeroLength::try_from(self.len()) {
            let tip_ptr = self.tip_ptr().unwrap();
            match len.try_into_inner_length() {
                Ok(len) => {
                    let inner = unsafe { tip_ptr.try_get_dirty(len)? };
                    Ok(Tip::Inner(inner))
                },
                Err(height) => {
                    let peak = unsafe { tip_ptr.try_get_dirty(height)? };
                    Ok(Tip::Peak(peak))
                },
            }
        } else {
            Ok(Tip::Empty)
        }
    }

    pub fn try_get_dirty_tip_mut(&mut self) -> Result<Tip<&mut SumPerfectTreeDyn<T, S, Z, P>,
                                                          &mut InnerDyn<T, S, Z, P>>,
                                                      P::Clean>
    {
        if let Ok(len) = NonZeroLength::try_from(self.len()) {
            let tip_ptr = self.tip_ptr_mut().unwrap();
            match len.try_into_inner_length() {
                Ok(len) => {
                    let inner = unsafe { tip_ptr.try_get_dirty_mut(len)? };
                    Ok(Tip::Inner(inner))
                },
                Err(height) => {
                    let peak = unsafe { tip_ptr.try_get_dirty_mut(height)? };
                    Ok(Tip::Peak(peak))
                },
            }
        } else {
            Ok(Tip::Empty)
        }
    }
}

pub enum InnerJoinError<T, S: Copy, Z: Zone> {
    HeightOverflow {
        left: SumPerfectTree<T, S, Z>,
        right: SumPerfectTree<T, S, Z>,
    },
    Peak(SumPerfectTree<T, S, Z>),
}

pub enum InnerPushPeakError<T, S: Copy, Z: Zone> {
    LengthOverflow(SumPerfectTree<T, S, Z>),
    Peak(SumPerfectTree<T, S, Z>),
    Zone(Z::Error),
}

impl<T, S: Copy, Z: Zone> Inner<T, S, Z>
where T: Load,
      S: Blob + Default
{
    /// Creates an `Inner` node by joining two trees.
    ///
    /// The trees must be *different* heights.
    pub fn try_join(left: SumPerfectTree<T, S, Z>, right: SumPerfectTree<T, S, Z>)
        -> Result<Self, InnerJoinError<T, S, Z>>
        where Z: Alloc
    {
        let (left, right) = match SumPerfectTree::try_join(left, right) {
            Ok(peak) => {
                return Err(InnerJoinError::Peak(peak));
            },
            Err(JoinError::HeightOverflow { lhs, rhs }) => {
                return Err(InnerJoinError::HeightOverflow { left: lhs, right: rhs });
            },
            Err(JoinError::HeightMismatch { lhs, rhs }) if lhs.height() > rhs.height() => {
                (lhs, rhs)
            },
            Err(JoinError::HeightMismatch { lhs, rhs }) => {
                (rhs, lhs)
            },
        };

        let len = left.len().checked_add(right.len()).unwrap();
        let len = InnerLength::new(len).unwrap();

        let left = SumMMR::from_peak(left);
        let right = SumMMR::from_peak(right);
        unsafe {
            Ok(Self::new_unchecked(left, right, len))
        }
    }

    /// Pushes a new tree.
    pub fn push_peak(
        self,
        peak: SumPerfectTree<T, S, Z>,
    ) -> Result<Self, InnerPushPeakError<T, S, Z>>
        where Z: Alloc + GetMut
    {
        //assert!(self.len.max_height() >= 
        match self.len.checked_add(peak.len()) {
            Ok(_new_len) => {
                todo!()
            },
            Err(Some(new_height)) => {
                todo!()
            },
            Err(None) => {
                Err(InnerPushPeakError::LengthOverflow(peak))
            }
        }
    }
}

impl<T, S: Copy, Z, P: Ptr, L: ToInnerLength> Inner<T, S, Z, P, L> {
    pub unsafe fn new_unchecked<LL, LR>(
        left: SumMMR<T, S, Z, P, LL>,
        right: SumMMR<T, S, Z, P, LR>,
        len: L
    ) -> Self
        where LL: ToLength,
              LR: ToLength,
    {
        Self {
            left: left.strip(),
            right: right.strip(),
            len
        }
    }
}

impl<T, S: Copy, Z, P: Ptr, L: ?Sized + ToInnerLength> Inner<T, S, Z, P, L> {
    pub fn len(&self) -> usize {
        self.len.to_length().into()
    }

    pub fn left(&self) -> &SumMMRDyn<T, S, Z, P> {
        let len = self.len.to_length();
        unsafe {
            &*SumMMRDyn::make_fat_ptr(&self.left as *const _ as *const _, len)
        }
    }

    pub fn left_mut(&mut self) -> &mut SumMMRDyn<T, S, Z, P> {
        let len = self.len.to_length();
        unsafe {
            &mut *SumMMRDyn::make_fat_ptr_mut(&mut self.left as *mut _ as *mut _, len)
        }
    }

    pub fn right(&self) -> &SumMMRDyn<T, S, Z, P> {
        let len = self.len.to_length();
        unsafe {
            &*SumMMRDyn::make_fat_ptr(&self.right as *const _ as *const _, len)
        }
    }

    pub fn right_mut(&mut self) -> &mut SumMMRDyn<T, S, Z, P> {
        let len = self.len.to_length();
        unsafe {
            &mut *SumMMRDyn::make_fat_ptr_mut(&mut self.right as *mut _ as *mut _, len)
        }
    }
}

// ------- unsizing related impls ------------

impl<T, S: Copy, Z, P: Ptr> Pointee for SumMMRDyn<T, S, Z, P> {
    type Metadata = Length;
    type LayoutError = !;

    fn metadata(ptr: *const Self) -> Self::Metadata {
        unsafe {
            let ptr: *const [()] = mem::transmute(ptr);
            ptr.len().into()
        }
    }

    fn make_fat_ptr(thin: *const (), length: Self::Metadata) -> *const Self {
        let ptr = ptr::slice_from_raw_parts(thin, length.into());
        unsafe { mem::transmute(ptr) }
    }

    fn make_fat_ptr_mut(thin: *mut (), length: Self::Metadata) -> *mut Self {
        let ptr = ptr::slice_from_raw_parts_mut(thin, length.into());
        unsafe { mem::transmute(ptr) }
    }
}

impl<T, S: Copy, Z, P: Ptr> Pointee for InnerDyn<T, S, Z, P> {
    type Metadata = InnerLength;
    type LayoutError = !;

    fn metadata(ptr: *const Self) -> Self::Metadata {
        unsafe {
            let ptr: *const [()] = mem::transmute(ptr);
            let len: usize = ptr.len();
            InnerLength::try_from(len)
                        .expect("valid metadata")
        }
    }

    fn make_fat_ptr(thin: *const (), length: Self::Metadata) -> *const Self {
        let ptr = ptr::slice_from_raw_parts(thin, length.into());
        unsafe { mem::transmute(ptr) }
    }

    fn make_fat_ptr_mut(thin: *mut (), length: Self::Metadata) -> *mut Self {
        let ptr = ptr::slice_from_raw_parts_mut(thin, length.into());
        unsafe { mem::transmute(ptr) }
    }
}

impl<T, S: Copy, Z, P: Ptr> Borrow<SumMMRDyn<T, S, Z, P>> for SumMMR<T, S, Z, P> {
    fn borrow(&self) -> &SumMMRDyn<T, S, Z, P> {
        unsafe {
            &*SumMMRDyn::make_fat_ptr(self as *const _ as *const (), self.len)
        }
    }
}

impl<T, S: Copy, Z, P: Ptr> BorrowMut<SumMMRDyn<T, S, Z, P>> for SumMMR<T, S, Z, P> {
    fn borrow_mut(&mut self) -> &mut SumMMRDyn<T, S, Z, P> {
        unsafe {
            &mut *SumMMRDyn::make_fat_ptr_mut(self as *mut _ as *mut (), self.len)
        }
    }
}

unsafe impl<T, S: Copy, Z, P: Ptr> Take<SumMMRDyn<T, S, Z, P>> for SumMMR<T, S, Z, P> {
    fn take_unsized<F, R>(self, f: F) -> R
        where F: FnOnce(Own<SumMMRDyn<T, S, Z, P>>) -> R
    {
        let mut this = ManuallyDrop::new(self);
        let this_dyn = this.deref_mut().borrow_mut();

        unsafe {
            f(Own::new_unchecked(this_dyn))
        }
    }
}

impl<T, S: Copy, Z, P: Ptr> IntoOwned for SumMMRDyn<T, S, Z, P> {
    type Owned = SumMMR<T, S, Z, P>;

    fn into_owned(self: Own<'_, Self>) -> Self::Owned {
        let this = Own::leak(self);

        unsafe {
            SumMMR {
                marker: PhantomData,
                zone: ptr::read(&this.zone),
                tip_ptr: ptr::read(&this.tip_ptr),
                tip_digest: ptr::read(&this.tip_digest),
                sum: ptr::read(&this.sum),
                len: this.len.to_length(),
            }
        }
    }
}

impl<T, S: Copy, Z, P: Ptr> Borrow<InnerDyn<T, S, Z, P>> for Inner<T, S, Z, P> {
    fn borrow(&self) -> &InnerDyn<T, S, Z, P> {
        unsafe {
            &*InnerDyn::make_fat_ptr(self as *const _ as *const (), self.len)
        }
    }
}

impl<T, S: Copy, Z, P: Ptr> BorrowMut<InnerDyn<T, S, Z, P>> for Inner<T, S, Z, P> {
    fn borrow_mut(&mut self) -> &mut InnerDyn<T, S, Z, P> {
        unsafe {
            &mut *InnerDyn::make_fat_ptr_mut(self as *mut _ as *mut (), self.len)
        }
    }
}

unsafe impl<T, S: Copy, Z, P: Ptr> Take<InnerDyn<T, S, Z, P>> for Inner<T, S, Z, P> {
    fn take_unsized<F, R>(self, f: F) -> R
        where F: FnOnce(Own<InnerDyn<T, S, Z, P>>) -> R
    {
        let mut this = ManuallyDrop::new(self);
        let this_dyn = this.deref_mut().borrow_mut();

        unsafe {
            f(Own::new_unchecked(this_dyn))
        }
    }
}

impl<T, S: Copy, Z, P: Ptr> IntoOwned for InnerDyn<T, S, Z, P> {
    type Owned = Inner<T, S, Z, P>;

    fn into_owned(self: Own<'_, Self>) -> Self::Owned {
        let this = Own::leak(self);

        unsafe {
            Inner {
                left: ptr::read(&this.left),
                right: ptr::read(&this.right),
                len: this.len.to_inner_length(),
            }
        }
    }
}

// --- hoard impls ---

#[derive(Debug, Error)]
#[error("FIXME")]
pub enum DecodeInnerBlobError<Peak: std::error::Error, Next: std::error::Error, Length: std::error::Error> {
    Peak(Peak),
    Next(Next),
    Length(Length),
}

impl<T, S: Copy, Z, P: PtrBlob, L: ToInnerLength> Blob for Inner<T, S, Z, P, L>
where T: Blob,
      S: Blob,
      Z: Blob,
      L: Blob,
{
    const SIZE: usize = <SumPerfectTree<T, S, Z, P, DummyHeight> as Blob>::SIZE +
                        <SumMMR<T, S, Z, P, DummyNonZeroLength> as Blob>::SIZE +
                        L::SIZE;

    type DecodeBytesError = DecodeInnerBlobError<!, !, !>;

    fn encode_bytes<'a>(&self, _: BytesUninit<'a, Self>) -> Bytes<'a, Self> { todo!() }

    fn decode_bytes(_: Bytes<'_, Self>) -> Result<MaybeValid<Self>, Self::DecodeBytesError> { todo!() }
}

unsafe impl<T, S: Copy, Z, P: PtrBlob> BlobDyn for InnerDyn<T, S, Z, P>
where T: Blob,
      S: Blob,
      Z: Blob,
{
    type DecodeBytesError = DecodeInnerBlobError<!, !, !>;

    fn try_size(_: <Self as Pointee>::Metadata) -> std::result::Result<usize, <Self as Pointee>::LayoutError> { todo!() }

    fn encode_bytes<'a>(&self, _: BytesUninit<'a, Self>) -> Bytes<'a, Self> { todo!() }

    fn decode_bytes(_: hoard::blob::Bytes<'_, Self>) -> std::result::Result<MaybeValid<<Self as IntoOwned>::Owned>, <Self as BlobDyn>::DecodeBytesError> { todo!() }
}

#[derive(Debug, Error)]
#[error("FIXME")]
pub enum DecodeSumMMRBytesError<
    Z: std::error::Error,
    P: std::error::Error,
    S: std::error::Error,
    L: std::error::Error,
>{
    Zone(Z),
    TipPtr(P),
    Sum(S),
    Len(L),
}

impl<T, S: Copy, Z, P: PtrBlob, L: ToLength> Blob for SumMMR<T, S, Z, P, L>
where T: Blob,
      S: Blob,
      Z: Blob,
      L: Blob,
{
    const SIZE: usize = Z::SIZE + P::SIZE + <Digest as Blob>::SIZE + S::SIZE + L::SIZE;
    type DecodeBytesError = DecodeSumMMRBytesError<Z::DecodeBytesError, P::DecodeBytesError, S::DecodeBytesError, L::DecodeBytesError>;

    fn encode_bytes<'a>(&self, _: BytesUninit<'a, Self>) -> Bytes<'a, Self> { todo!() }

    fn decode_bytes(_: Bytes<'_, Self>) -> Result<MaybeValid<Self>, Self::DecodeBytesError> { todo!() }
}

unsafe impl<T, S: Copy, Z, P: PtrBlob> BlobDyn for SumMMRDyn<T, S, Z, P>
where T: Blob,
      S: Blob,
      Z: Blob,
{
    type DecodeBytesError = DecodeSumMMRBytesError<Z::DecodeBytesError, P::DecodeBytesError, S::DecodeBytesError, !>;

    fn try_size(_: <Self as Pointee>::Metadata) -> std::result::Result<usize, <Self as Pointee>::LayoutError> { todo!() }

    fn encode_bytes<'a>(&self, _: BytesUninit<'a, Self>) -> Bytes<'a, Self> { todo!() }

    fn decode_bytes(_: hoard::blob::Bytes<'_, Self>) -> std::result::Result<MaybeValid<<Self as IntoOwned>::Owned>, <Self as BlobDyn>::DecodeBytesError> { todo!() }
}

impl<T, S: Copy, Z: Zone, P: Ptr, L: ToLength> Load for SumMMR<T, S, Z, P, L>
where T: Load,
      S: Blob,
      L: Blob,
{
    type Blob = SumMMR<T::Blob, S, (), P::Blob, L>;
    type Zone = Z;

    fn load(_blob: Self::Blob, _zone: &<Self as Load>::Zone) -> Self {
        todo!()
    }
}

impl<T, S: Copy, Z: Zone, P: Ptr> LoadRef for SumMMRDyn<T, S, Z, P>
where T: Load,
      S: Blob,
{
    type BlobDyn = SumMMRDyn<T::Blob, S, (), P::Blob>;
    type Zone = Z;

    fn load_ref_from_bytes<'a>(_: hoard::blob::Bytes<'a, <Self as LoadRef>::BlobDyn>, _: &<Self as LoadRef>::Zone) -> std::result::Result<MaybeValid<hoard::owned::Ref<'a, Self>>, <<Self as LoadRef>::BlobDyn as BlobDyn>::DecodeBytesError> { todo!() }
}

impl<T, S: Copy, Z: Zone, P: Ptr, L: ToInnerLength> Load for Inner<T, S, Z, P, L>
where T: Load,
      S: Blob,
      L: Blob,
{
    type Blob = Inner<T::Blob, S, (), P::Blob, L>;
    type Zone = Z;

    fn load(_blob: Self::Blob, _zone: &<Self as Load>::Zone) -> Self {
        todo!()
    }
}

impl<T, S: Copy, Z: Zone, P: Ptr> LoadRef for InnerDyn<T, S, Z, P>
where T: Load,
      S: Blob,
{
    type BlobDyn = InnerDyn<T::Blob, S, (), P::Blob>;
    type Zone = Z;

    fn load_ref_from_bytes<'a>(_: hoard::blob::Bytes<'a, <Self as LoadRef>::BlobDyn>, _: &<Self as LoadRef>::Zone) -> std::result::Result<MaybeValid<hoard::owned::Ref<'a, Self>>, <<Self as LoadRef>::BlobDyn as BlobDyn>::DecodeBytesError> { todo!() }
}

#[cfg(test)]
mod tests {
    use super::*;

    use hoard::pile::PileMut;

    #[test]
    fn mmr_push() {
        let pile = PileMut::<[u8]>::default();

        let mut mmr = MMR::new_in(pile);

        dbg!(&mmr);
        dbg!(mmr.try_push(1u8));
        dbg!(&mmr);
        dbg!(mmr.try_push(2u8));
        dbg!(&mmr);

        dbg!(mmr.try_push(3u8));
        dbg!(&mmr);

        dbg!(mmr.try_push(4u8));
        dbg!(&mmr);
    }
}
*/
