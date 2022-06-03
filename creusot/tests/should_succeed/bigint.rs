extern crate creusot_contracts;
extern crate num_bigint;
use creusot_contracts::*;

use num_bigint::BigInt;
use std::ops::Add;


// extern_spec! {
//     mod num_bigint {
//         impl<'a> Add<&'a BigInt> for BigInt {
//             // #[ensures(@result == @self + @rhs)]
//             fn add(self, rhs: &'a BigInt) -> BigInt;
//         }
//     }
// }


fn omg() {
  let a = BigInt::from(0);
  let b = BigInt::from(0);

  let _ = a + b;
  // assert!(a == b);
}
