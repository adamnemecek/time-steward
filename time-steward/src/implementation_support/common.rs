use super::super::api::*;
use std::borrow::Borrow;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Debug;
use std::ops::Deref;
use std::rc::Rc;
use std::any::Any;
use serde::{Serialize, Serializer, Deserialize, Deserializer};
use crate::DeterministicRandomId;

pub fn split_off_greater<K: Ord + Borrow<Q> + Clone, V, Q: Ord + ?Sized>(
  input: &mut BTreeMap<K, V>,
  split: &Q,
) -> BTreeMap<K, V> {
  // BTreeMap::split_off() DOES remove this splitting key, while we want to NOT include that key.
  // TODO: will Rust eventually make this easier?
  let mut result = input.split_off(split);
  let mut transfer = None;
  if let Some(whoops) = result.iter().next() {
    if whoops.0.borrow() == split {
      transfer = Some(whoops.0.clone());
    }
  }
  if let Some(key) = transfer {
    input.insert(key, result.remove(split).unwrap());
  }
  result
}

pub fn split_off_greater_set<K: Ord + Borrow<Q>, Q: Ord + ?Sized>(
  input: &mut BTreeSet<K>,
  split: &Q,
) -> BTreeSet<K> {
  // BTreeMap::split_off() DOES remove this splitting key, while we want to NOT include that key.
  // TODO: will Rust eventually make this easier?
  let mut result = input.split_off(split);
  if let Some(whoops) = result.take(split) {
    input.insert(whoops);
  }
  #[cfg(debug_assertions)]
  {
    if result.take(split).is_some() {
      panic!("Some code broke the Ord/Borrow rules for BTreeSet calls")
    }
  }
  result
}

/*macro_rules! downcast_rc {
  ($input: expr, $T: ty, $($Trait:tt)*) => {{
    let result: Result <Rc<$T>, Rc<$($Trait)*>> = {
      let input = $input;
      if (*input).get_type_id() == ::std::any::TypeId::of::<$T>() {
        //println!( "succeeded");
        unsafe {
          let raw: ::std::raw::TraitObject = ::std::mem::transmute (input);
          Ok(::std::mem::transmute (raw.data))
        }
      }
      else {
        Err (input)
      }
    };
    result
  }}
}*/
macro_rules! downcast_ref {
  ($input: expr, $T: ty, $($Trait:tt)*) => {{
    let result: Option<&$T> = {
      let input = $input;
      if (*input).get_type_id() == ::std::any::TypeId::of::<$T>() {
        //println!( "succeeded");
        unsafe {
          let raw: ::std::raw::TraitObject = ::std::mem::transmute(input);
          Some(::std::mem::transmute(raw.data))
        }
      } else {
        None
      }
    };
    result
  }};
}


#[doc(hidden)]
#[macro_export]
macro_rules! delegate {
  (Ord, $this: ident => $target: expr, [$($bounds:tt)*], [$($concrete:tt)*]) => {
    impl<$($bounds)*> Ord for $($concrete)* {
      fn cmp(&self, other: &Self) -> Ordering {
        let foo = { let $this = self; $target };
        let bar = { let $this = other; $target };
        foo.cmp(bar)
      }
    }
  };
  (PartialOrd, $this: ident => $target: expr, [$($bounds:tt)*], [$($concrete:tt)*]) => {
    impl<$($bounds)*> PartialOrd for $($concrete)* {
      fn partial_cmp(&self, other: &Self) ->Option <Ordering> {
        let foo = { let $this = self; $target };
        let bar = { let $this = other; $target };
        foo.partial_cmp(bar)
      }
    }
  };
  (Eq, $this: ident => $target: expr, [$($bounds:tt)*], [$($concrete:tt)*]) => {
    impl<$($bounds)*> Eq for $($concrete)* {}
  };
  (PartialEq, $this: ident => $target: expr, [$($bounds:tt)*], [$($concrete:tt)*]) => {
    impl<$($bounds)*> PartialEq for $($concrete)* {
      fn eq(&self, other: &Self) -> bool {
        let foo = { let $this = self; $target };
        let bar = { let $this = other; $target };
        foo.eq(bar)
      }
    }
  };
  (Hash, $this: ident => $target: expr, [$($bounds:tt)*], [$($concrete:tt)*]) => {
    impl<$($bounds)*> ::std::hash::Hash for $($concrete)* {
      fn hash <H: ::std::hash::Hasher> (&self, state: &mut H) {
        let foo = { let $this = self; $target };
        foo.hash (state);
      }
    }
  };
  ($Trait1: tt, $Trait2: tt, [$this: ident => $target: expr], [$($bounds:tt)*], [$($concrete:tt)*]) => {
    delegate! ($Trait1, $this => $target, [$($bounds)*], [$($concrete)*]);
    delegate! ($Trait2, $this => $target, [$($bounds)*], [$($concrete)*]);
  };
  ($Trait1: tt, $Trait2: tt, $Trait3: tt, [$this: ident => $target: expr], [$($bounds:tt)*], [$($concrete:tt)*]) => {
    delegate! ($Trait1, $this => $target, [$($bounds)*], [$($concrete)*]);
    delegate! ($Trait2, $this => $target, [$($bounds)*], [$($concrete)*]);
    delegate! ($Trait3, $this => $target, [$($bounds)*], [$($concrete)*]);
  };
  ($Trait1: tt, $Trait2: tt, $Trait3: tt, $Trait4: tt, [$this: ident => $target: expr], [$($bounds:tt)*], [$($concrete:tt)*]) => {
    delegate! ($Trait1, $this => $target, [$($bounds)*], [$($concrete)*]);
    delegate! ($Trait2, $this => $target, [$($bounds)*], [$($concrete)*]);
    delegate! ($Trait3, $this => $target, [$($bounds)*], [$($concrete)*]);
    delegate! ($Trait4, $this => $target, [$($bounds)*], [$($concrete)*]);
  };
  ($Trait1: tt, $Trait2: tt, $Trait3: tt, $Trait4: tt, $Trait5: tt, [$this: ident => $target: expr], [$($bounds:tt)*], [$($concrete:tt)*]) => {
    delegate! ($Trait1, $this => $target, [$($bounds)*], [$($concrete)*]);
    delegate! ($Trait2, $this => $target, [$($bounds)*], [$($concrete)*]);
    delegate! ($Trait3, $this => $target, [$($bounds)*], [$($concrete)*]);
    delegate! ($Trait4, $this => $target, [$($bounds)*], [$($concrete)*]);
    delegate! ($Trait5, $this => $target, [$($bounds)*], [$($concrete)*]);
  };
}

pub trait PrivateTimeStewardDataTrait: Any + Debug {}
impl <T: Any + Debug> PrivateTimeStewardDataTrait for T {}

#[derive(Derivative)]
#[derivative(Clone(bound = ""))]
pub struct DataHandle <PublicImmutableData, PrivateTimeStewardData> (Rc<(PublicImmutableData, PrivateTimeStewardData)>);

impl <PublicImmutableData: SimulationStateData, PrivateTimeStewardData: PrivateTimeStewardDataTrait> DataHandleTrait for DataHandle<PublicImmutableData, PrivateTimeStewardData> {}

impl <PublicImmutableData: SimulationStateData, PrivateTimeStewardData: PrivateTimeStewardDataTrait> Deref for DataHandle<PublicImmutableData, PrivateTimeStewardData> {
  type Target = PublicImmutableData;
  fn deref (&self)->& Self::Target {
    & (self.0).0
  }
}

delegate! (PartialEq, Eq, Hash, [this => &(&*this as *const _ as usize)], [PublicImmutableData: SimulationStateData, PrivateTimeStewardData: PrivateTimeStewardDataTrait], [DataHandle<PublicImmutableData, PrivateTimeStewardData>]);

#[derive (Clone, Debug)]
pub struct TimeOrderedEventHandle <Steward: TimeSteward> (Steward::EventHandle);

delegate! (PartialEq, Eq, PartialOrd, Ord, Hash, [this => this.0.extended_time()], [Steward: TimeSteward], [TimeOrderedEventHandle <Steward>]);

impl <Steward: TimeSteward> Borrow<ExtendedTime <Steward::SimulationSpec>> for TimeOrderedEventHandle <Steward> {
  fn borrow (&self)->& ExtendedTime <Steward::SimulationSpec> {self.0.extended_time()}
}

impl <PublicImmutableData: SimulationStateData, PrivateTimeStewardData: PrivateTimeStewardDataTrait> Debug for DataHandle<PublicImmutableData, PrivateTimeStewardData> {
  fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
    write!(f, "DataHandle(@{:p})", self.0)
  }
}

impl <PublicImmutableData: SimulationStateData, PrivateTimeStewardData: PrivateTimeStewardDataTrait> Serialize for DataHandle<PublicImmutableData, PrivateTimeStewardData> {
  fn serialize <S: Serializer> (&self, _serializer: S)->Result <S::Ok, S::Error> {
    unimplemented!()
  }
}

impl <'a, PublicImmutableData: SimulationStateData, PrivateTimeStewardData: PrivateTimeStewardDataTrait> Deserialize<'a> for DataHandle<PublicImmutableData, PrivateTimeStewardData> {
  fn deserialize <D: Deserializer<'a>> (_deserializer: D)->Result <Self, D::Error> {
    unimplemented!()
  }
}


/*pub trait DeserializationContext {
fn deserialize_data <T: DeserializeOwned> (&mut self)->T;
fn deserialize_timeline_handle <T: Entity> (&mut self)->EntityHandle <T>;
fn deserialize_prediction_handle <T: Event> (&mut self)->PredictionHandle <T>;
fn deserialize_event_handle <T: Event> (&mut self)->EventHandle <T>;
fn deserialize_dynamic_event_handle (&mut self)->DynamicEventHandle;
}*/

pub fn extended_time_of_fiat_event<S: SimulationSpec>(
  time: S::Time,
  id: DeterministicRandomId,
) -> ExtendedTime<S> {
  ExtendedTime {
    base: time,
    iteration: 0,
    id: id.for_fiat_event_internal(),
  }
}

pub fn extended_time_of_predicted_event<S: SimulationSpec>(
  event_base_time: S::Time,
  id: DeterministicRandomId,
  from: &ExtendedTime<S>,
) -> Option<ExtendedTime<S>> {
  let iteration = match event_base_time.cmp(&from.base) {
    Ordering::Less => return None, // short-circuit
    Ordering::Greater => 0,
    Ordering::Equal => {
      if id > from.id {
        from.iteration
      } else {
        if from.iteration >= S::MAX_ITERATION {
          panic!("Too many iterations at the same base time; probably an infinite loop")
        }
        from.iteration + 1
      }
    }
  };
  Some(ExtendedTime {
    base: event_base_time,
    iteration: iteration,
    id: id,
  })
}

#[derive(Debug)]
pub struct EventChildrenIdGenerator {
  next: Option <DeterministicRandomId>
}

impl EventChildrenIdGenerator {
  pub fn new()->EventChildrenIdGenerator {EventChildrenIdGenerator {next: None}}
  pub fn next(&mut self, this_event_id: & DeterministicRandomId)->DeterministicRandomId {
    let result = match self.next {
      None => DeterministicRandomId::new (this_event_id),
      Some (next) => next,
    };
    self.next = Some (DeterministicRandomId::from_raw ([
      result.data() [0], result.data() [1].wrapping_add (1)
    ]));
    result
  }
}
