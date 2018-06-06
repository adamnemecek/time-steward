use num::{PrimInt, Signed};
use std::mem;
use std::cmp::{min};

/// Right-shift an integer, but round to nearest, with ties rounding to even.
///
/// This minimizes error, and avoids a directional bias.
pub fn right_shift_nicely_rounded <T: PrimInt> (input: T, shift: usize)->T {
  let divisor = T::one() << shift;
  let mask = divisor - T::one();
  if (input & mask) << 1 == divisor {
    let shifted = input >> shift;
    shifted + (shifted & T::one())
  } else {
    (input + (divisor >> 1)) >> shift
  }
}


/// Approximately evaluate an integer polynomial at an input in the range [-0.5, 0.5].
///
/// The input is represented as an integer combined with a right-shift size.
/// Each coefficient, except the constant one, must obey coefficient.abs() <= T::max_value() >> shift.
/// The constant coefficient must obey coefficient.abs() <= T::max_value() - (T::max_value() >> shift),
/// i.e. be small enough that you can add one of the other coefficients to it without overflowing.
/// Given these conditions, the result is guaranteed to be strictly within 1 of the ideal result.
/// For instance, if the ideal result was 2.125, this function could return 2 or 3,
/// but if it was 2, this function can only return exactly 2.
pub fn evaluate_polynomial_at_small_input <Coefficient: Copy, T: PrimInt + Signed + From <Coefficient>> (coefficients: & [Coefficient], input: T, shift: usize)->T {
  let half = (T::one() << shift) >> 1;
  assert!(-half <= input && input <= half, "inputs to evaluate_polynomial_at_small_input must be in the range [-0.5, 0.5]");
  
  if coefficients.len () == 0 {return T::zero()}
    
  let mut result = T::zero();
  for coefficient in coefficients.iter().skip(1).rev() {
    result = right_shift_nicely_rounded ((result + (*coefficient).into())*input, shift);
  }
  result + coefficients [0].into()
}

/// Translate a polynomial horizontally, letting it take inputs relative to a given input instead of relative to 0.
///
/// This is equivalent to replacing its coefficients with its Taylor coefficients at the given input.
/// This is equivalent to translating the graph *left* by the given input amount.
///
/// For constant polynomials, this is a no-op.
///
/// Otherwise, to avoid overflow, each term, when evaluating at the given input,
/// must obey term.abs() <= T::max_value() >> (coefficients.len() - 1).
/// If this condition is broken, translate_polynomial() returns Err and makes no changes.
pub fn translate_polynomial <T: PrimInt + Signed> (coefficients: &mut [T], input: T)->Result <(),()> {
  if coefficients.len() <= 1 {return Ok(());}
  if !safe_to_translate_polynomial_to (coefficients, input, |_| T::max_value()) {return Err (())}
  translate_polynomial_unchecked (coefficients, input) ;
  Ok (())
}

/// Same as translate_polynomial(), but assume no overflow will occur.
///
/// Useful in performance-critical code when you already know there won't be overflow,
/// such as in the range returned by conservative_safe_polynomial_translation_range.
pub fn translate_polynomial_unchecked <T: PrimInt + Signed> (coefficients: &mut [T], input: T) {
  coefficients.reverse();
  for index in 0..coefficients.len() {
    let coefficient = mem::replace (&mut coefficients [index], T::zero());
    for derivative in (1..(index + 1)).rev() {
      coefficients [derivative] = coefficients [derivative]*input + coefficients [derivative - 1]
    }
    coefficients [0] = coefficients [0]*input + coefficient
  }
}

pub fn safe_to_translate_polynomial_to <T: PrimInt + Signed, MaximumFn: Fn (usize)->T> (coefficients: & [T], input: T, maximum: MaximumFn)->bool {
  if coefficients.len() <= 1 {return true;}
  let mut factor = T::one();
  let input = input.abs();
  let global_maximum = T::max_value() >> (coefficients.len() - 1) ;
  for (index, coefficient) in coefficients.iter().enumerate() {
    let maximum = min (global_maximum, maximum (index));
    if coefficient == &T::min_value() {return false;}
    match coefficient.abs().checked_mul (&factor) {
      None => return false,
      Some (term) => if term > maximum {return false;},
    }
    if index + 1 < coefficients.len() { match factor.checked_mul (&input) {
      None => return false,
      Some (next_factor) => factor = next_factor,
    }}
  }
  true
}

/// Find the range of inputs to which the polynomial can safely be translated.
///
/// Anything in the range [-result, result] is safe to translate to; anything outside isn't.
///
/// The third argument can impose a stricter maximum on the resulting coefficients.
pub fn exact_safe_polynomial_translation_range <T: PrimInt + Signed, MaximumFn: Fn (usize)->T> (coefficients: & [T], maximum: MaximumFn)->Option <T> {
  if coefficients.len() <= 1 || coefficients [1..].iter().all (| coefficient | coefficient == &T::zero()) {return Some (T::max_value())}
  // if we know that there's at least one nonzero non-constant coefficient, T::max_value is never legal, so we can pick a min that definitely legal and a max that's definitely not
  let mut min = -T::one();
  let mut max = T::max_value();
  while min + T::one() < max {
    let mid = (min >> 1) + (max >> 1) + (min & max & T::one());
    if safe_to_translate_polynomial_to (coefficients, mid, & maximum) {
      min = mid;
    } else {
      max = mid;
    }
  }
  if min < T::zero() {None}
  else {Some (min)}
}

/// Find a range of inputs to which the polynomial can safely be translated.
///
/// Anything in the range [-result, result] is safe to translate to.
/// Anything outside the range (-result*2, result*2) isn't.
///
/// The third argument can impose a stricter maximum on the resulting coefficients.
///
/// This function is much faster than exact_safe_polynomial_translation_range, at the cost of being less precise.
pub fn conservative_safe_polynomial_translation_range <T: PrimInt + Signed, MaximumFn: Fn (usize)->T> (coefficients: &mut [T], maximum: MaximumFn)->Option <T> {
  if coefficients.len() <= 1 {return Some (T::max_value())}
  let global_maximum = T::max_value() >> (coefficients.len() - 1) ;
  let mut result_shift = mem::size_of::<T>()*8 - 2;
  for (power, coefficient) in coefficients.iter().enumerate() {
    let magnitude = coefficient.abs();
    let maximum = min (global_maximum, maximum (power));
    while magnitude > (maximum >> (result_shift*power)) {
      if result_shift == 0 {return None;}
      result_shift -= 1;
    }
  }
  Some (T::one() << result_shift)
}

#[cfg (test)]
mod tests {
  use super::*;
  use num::{Zero, One, FromPrimitive};
  use num::bigint::BigInt;
  use num::rational::{Ratio, BigRational};
  use rand::distributions::Distribution;  
  use rand::distributions::range::{Range};
  use rand::{Rng, SeedableRng};
  
  fn evaluate_polynomial_exactly <Coefficient: Copy, T: PrimInt + Signed + From <Coefficient>> (coefficients: & [Coefficient], input: T, shift: usize)->BigRational
    where BigInt: From <Coefficient> + From <T> {
    let mut result: BigRational = Ratio::zero();
    let input = Ratio::new(BigInt::from (input), BigInt::one() << shift);
    for coefficient in coefficients.iter().rev() {
      result = result*&input + BigInt::from(*coefficient);
    }
    result
  }
  
  #[test]
  fn test_right_shift_nicely_rounded() {
    let inputs: Vec<(i64, usize, i64)> = vec![
      (0, 0, 0), (0, 5, 0), (1, 3, 0), (4, 3, 0), (5, 3, 1),
      (999, 1, 500), (998, 1, 499), (997, 1, 498)
    ];
    for (input, shift, result) in inputs {
      assert_eq!(right_shift_nicely_rounded (input, shift), result);
      assert_eq!(right_shift_nicely_rounded (-input, shift), -result);
    }
  }
  
  #[test]
  fn test_evaluate_polynomial_at_small_input() {
    let mut generator =::rand::chacha::ChaChaRng::from_seed ([33; 32]) ;
    for shift in 0..5 {
      let shift = 1 << shift;
      let half = 1i64 << (shift - 1);
      let maximum = i64::max_value() >> shift;
      let constant_maximum = i64::max_value() - maximum;
      let range = Range::new_inclusive (- maximum, maximum) ;
      let constant_range = Range::new_inclusive (- constant_maximum, constant_maximum);
      let mut test_inputs = vec![- half, - half + 1, - 1, 0, 1, half - 1, half];
      if half > 2 {for _ in 0..6 {test_inputs.push (generator.gen_range (- half + 2, half - 1));}}
      for _ in 0..40 {
        let mut coefficients = vec![constant_range.sample (&mut generator)];
        while generator.gen() {coefficients.push (range.sample (&mut generator));}
        for input in test_inputs.iter() {
          let result = evaluate_polynomial_at_small_input (& coefficients,*input, shift);
          
          let perfect_result = evaluate_polynomial_exactly (& coefficients,*input, shift);
          let difference = Ratio::from_integer (FromPrimitive::from_i64 (result).unwrap()) - perfect_result;
          assert!(difference < Ratio::from_integer (FromPrimitive::from_i64 (1).unwrap()));
        }
      }
    }
  }
}
