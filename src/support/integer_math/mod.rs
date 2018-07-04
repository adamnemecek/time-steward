use num::{self};
use num::traits::{Zero, WrappingAdd, WrappingSub, WrappingMul, CheckedShl, CheckedShr, Signed};
use std::mem;
use std::cmp::{min};
use std::ops::{Add, Sub, Mul, AddAssign, SubAssign, MulAssign, Shl, Shr, Neg, Index};
use std::fmt::Debug;

pub trait Integer:
  'static + num::PrimInt + num::Integer + num::FromPrimitive +
  AddAssign <Self> + SubAssign <Self> + MulAssign <Self> +
  WrappingAdd + WrappingSub + WrappingMul +
  for<'a> Add <&'a Self, Output = Self> + for<'a> Sub <&'a Self, Output = Self> + for<'a> Mul <&'a Self, Output = Self> +
  CheckedShl + CheckedShr + Shl <u32, Output = Self> + Shr <u32, Output = Self> +
  Debug {}
impl <T: 'static + num::PrimInt + num::Integer + num::FromPrimitive + AddAssign <Self> + SubAssign <Self> + MulAssign <Self> + WrappingAdd + WrappingSub + WrappingMul + for<'a> Add <&'a Self, Output = Self> + for<'a> Sub <&'a Self, Output = Self> + for<'a> Mul <&'a Self, Output = Self> + CheckedShl + CheckedShr + Shl <u32, Output = Self> + Shr <u32, Output = Self> + Debug> Integer for T {}


pub trait HasCoordinates
   {
  type Coordinate: Integer + Signed + Vector + HasCoordinates <Coordinate = Self::Coordinate>;}

pub trait Vector:
  'static + Sized + Copy + Clone + HasCoordinates +
  Add <Self, Output = Self> + Sub <Self, Output = Self> + Mul <<Self as HasCoordinates>::Coordinate, Output = Self> +
  for<'a> Add <&'a Self, Output = Self> + for<'a> Sub <&'a Self, Output = Self> + //for<'a> Mul <&'a <Self as HasCoordinates>::Coordinate, Output = Self> +
  AddAssign <Self> + SubAssign <Self> + MulAssign <<Self as HasCoordinates>::Coordinate> +
  for<'a> AddAssign <&'a Self> + for<'a> SubAssign <&'a Self> + //for<'a> MulAssign <&'a <Self as HasCoordinates>::Coordinate> +
  Zero + Neg <Output = Self>
  {
  const DIMENSIONS: usize;
  fn coordinate (&self, which: usize)->Self::Coordinate;
  fn set_coordinate (&mut self, which: usize, value: Self::Coordinate);
  fn map_coordinates <F: Fn(Self::Coordinate)->Self::Coordinate> (mut self, transform: F)->Self {
    for index in 0..Self::DIMENSIONS {
      let new_coordinate = (transform) (self.coordinate (index));
      self.set_coordinate (index, new_coordinate);
    }
    self
  }
}

pub mod impls {
  use super::*;
  use nalgebra::*;
  macro_rules! impl_vector {
    ($([$coordinates: expr, $Vector: ident],)*) => {
      $(
        impl <T: Integer + Signed + Vector + HasCoordinates <Coordinate = T>> HasCoordinates for $Vector <T> {type Coordinate = T;}
        impl <T: Integer + Signed + Vector + HasCoordinates <Coordinate = T>> Vector for $Vector <T> {
          const DIMENSIONS: usize = $coordinates;
          fn coordinate (&self, which: usize)->Self::Coordinate {self [which]}
          fn set_coordinate (&mut self, which: usize, value: Self::Coordinate) {self [which] = value}
        }
      )*
    }
  }
  macro_rules! impl_integer {
    ($($Integer: ident,)*) => {
      $(
        impl HasCoordinates for $Integer {type Coordinate = $Integer;}
        impl Vector for $Integer {
          const DIMENSIONS: usize = 1;
          fn coordinate (&self, which: usize)->Self::Coordinate {*self}
          fn set_coordinate (&mut self, which: usize, value: Self::Coordinate) {*self = value}
        }
      )*
    }
  }
  impl_vector! ([1, Vector1], [2, Vector2], [3, Vector3], [4, Vector4], [5, Vector5], [6, Vector6],);
  impl_integer! (i8, i16, i32, i64, isize,);
}

/// Right-shift an integer, but round to nearest, with ties rounding to even.
///
/// This minimizes error, and avoids a directional bias.
pub fn shr_nicely_rounded <T: Integer> (input: T, shift: u32)->T {
  let divisor = match T::one().checked_shl ( shift ) {Some (value) => value, None => return T::zero()};
  let mask = divisor.wrapping_sub (&T::one());
  let shifted = input >> shift;
  let half = divisor >> 1u32;
  shifted + if (input & mask)+(shifted & if shift != 0 {T::one()} else {T::zero()}) > half {T::one()} else {T::zero()}
}

/// Right-shift an integer, but round to even.
///
/// This avoids a directional bias.
pub fn shr_round_to_even <T: Integer> (input: T, shift: u32)->T {
  let divisor = match T::one().checked_shl ( shift ) {Some (value) => value, None => return T::zero()};
  let mask = divisor.wrapping_sub (&T::one());
  let shifted = input >> shift;
  shifted + if (input & mask) != T::zero() {shifted & T::one()} else {T::zero()}
}

/// Right-shift an integer, but round towards positive infinity.
pub fn shr_ceil <T: Integer> (input: T, shift: u32)->T {
  let divisor = match T::one().checked_shl ( shift ) {Some (value) => value, None => return T::zero()};
  let mask = divisor.wrapping_sub (&T::one());
  (input >> shift) + if input & mask != T::zero() {T::one()} else {T::zero()}
}

/// Right-shift an integer, but round towards 0.
pub fn shr_round_towards_zero <T: Integer> (input: T, shift: u32)->T {
  (input + if input <T::zero() {(T::one() << shift).wrapping_sub (& T::one())} else {T::zero()}) >> shift
}

/// Left-shift an integer, returning Some(input*(2^shift)) if it fits within the type, None otherwise.
pub fn overflow_checked_shl <T: Integer> (input: T, shift: u32)->Option <T> {
  if input == T::zero() {return Some (T::zero())}
  let maximum = match T::max_value().checked_shr (shift) {
    None => return None,
    Some (value) => value,
  };
  if input > maximum {return None}
  let minimum = T::min_value() >> shift;
  if input < minimum {return None}
  Some (input << shift)
}

/// Compute the arithmetic mean of two integers, rounded towards negative infinity. Never overflows.
pub fn mean_floor <T: Integer> (first: T, second: T)->T {
  (first >> 1u32) + (second >> 1u32) + (first & second & T::one())
}

/// Compute the arithmetic mean of two integers, rounded towards positive infinity. Never overflows.
pub fn mean_ceil <T: Integer> (first: T, second: T)->T {
  (first >> 1u32) + (second >> 1u32) + ((first | second) & T::one())
}


pub mod polynomial;

#[cfg (test)]
mod tests {
  use super::*;
  use num::{One, ToPrimitive, Integer};
  use num::bigint::BigInt;
  use num::rational::{Ratio};
  use std::cmp::Ordering;
    
  fn perfect_shr_nicely_rounded <T: Integer> (input: T, shift: u32)->BigInt where BigInt: From <T> {
    let perfect_result = Ratio::new (BigInt::from (input), BigInt::one() << shift as usize);
    let rounded_down = perfect_result.floor();
    let fraction = & perfect_result - & rounded_down;
    let rounded_down = rounded_down.to_integer();
    match fraction.cmp (& Ratio::new (BigInt::one(), BigInt::one() << 1)) {
      Ordering::Less => rounded_down, Ordering::Greater => rounded_down + BigInt::one() ,
      Ordering::Equal => & rounded_down + rounded_down.mod_floor (& (BigInt::one() << 1)),
    }
  }
  
  fn perfect_shr_round_to_even <T: Integer> (input: T, shift: u32)->BigInt where BigInt: From <T> {
    let perfect_result = Ratio::new (BigInt::from (input), BigInt::one() << shift as usize);
    let rounded_down = perfect_result.floor();
    if perfect_result == rounded_down {rounded_down.to_integer()}
    else {
      let rounded_down = rounded_down.to_integer();
      if &rounded_down >> 1 << 1 == rounded_down {
        rounded_down
      }
      else {rounded_down + 1}
    }
  }
  
  #[test]
  fn test_shr_nicely_rounded() {
    let inputs: Vec<(i64, u32, i64)> = vec![
      (0, 0, 0), (0, 5, 0), (5, 0, 5), (1, 3, 0), (4, 3, 0), (5, 3, 1),
      (999, 1, 500), (998, 1, 499), (997, 1, 498)
    ];
    for (input, shift, result) in inputs {
      println!( "{:?}", (input, shift, result));
      assert_eq!(shr_nicely_rounded (input, shift), result);
      assert_eq!(shr_nicely_rounded (-input, shift), -result);
    }
  }
  
  #[test]
  fn test_shr_round_to_even() {
    let inputs: Vec<(i64, u32, i64)> = vec![
      (0, 0, 0), (0, 5, 0), (5, 0, 5), (1, 3, 0), (4, 3, 0), (5, 3, 0),
      (999, 1, 500), (998, 1, 499), (997, 1, 498)
    ];
    for (input, shift, result) in inputs {
      println!( "{:?}", (input, shift, result));
      assert_eq!(shr_round_to_even (input, shift), result);
      assert_eq!(shr_round_to_even (-input, shift), -result);
    }
  }
  
  quickcheck! {
    fn quickcheck_shr_nicely_rounded_signed (input: i32, shift: u8)->bool {
      let result = shr_nicely_rounded (input, shift as u32);
      let perfect_result = perfect_shr_nicely_rounded (input, shift as u32);
      println!( "{:?}", (result, & perfect_result.to_str_radix (10)));
      perfect_result == BigInt::from (result)
    }
    
    fn quickcheck_shr_nicely_rounded_unsigned (input: u32, shift: u8)->bool {
      let result = shr_nicely_rounded (input, shift as u32);
      let perfect_result = perfect_shr_nicely_rounded (input, shift as u32);
      println!( "{:?}", (result, & perfect_result.to_str_radix (10)));
      perfect_result == BigInt::from (result)
    }
    
    fn quickcheck_shr_round_to_even_signed (input: i32, shift: u8)->bool {
      let result = shr_round_to_even (input, shift as u32);
      let perfect_result = perfect_shr_round_to_even (input, shift as u32);
      println!( "{:?}", (result, & perfect_result.to_str_radix (10)));
      perfect_result == BigInt::from (result)
    }
    
    fn quickcheck_shr_round_to_even_unsigned (input: u32, shift: u8)->bool {
      let result = shr_round_to_even (input, shift as u32);
      let perfect_result = perfect_shr_round_to_even (input, shift as u32);
      println!( "{:?}", (result, & perfect_result.to_str_radix (10)));
      perfect_result == BigInt::from (result)
    }
    
    fn quickcheck_overflow_checked_shl (input: i32, shift: u8)->bool {
      let result = overflow_checked_shl (input, shift as u32);
      let perfect_result = BigInt::from (input) << shift as usize;
      result == perfect_result.to_i32()
    }
  }
}
