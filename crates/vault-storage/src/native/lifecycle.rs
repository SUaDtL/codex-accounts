//! CA-04B native primitives, inside the existing reviewed unsafe boundary.
//! There is no public path/process executor or production Effects adapter.
//! Synthetic child construction exists only in lifecycle_tests.rs.
#![deny(unsafe_op_in_unsafe_fn)]
mod helper;
mod home;
mod process;

#[cfg(test)]
#[path = "lifecycle_tests.rs"]
mod tests;

// Test-only construction; never a production filesystem authority.
mod target;
#[cfg(test)]
mod target_tests;
