use super::super::api::*;
use std::cmp::Ordering;
use ::DeterministicRandomId;

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
    let result: Option <&$T> = {
      let input = $input;
      if (*input).get_type_id() == ::std::any::TypeId::of::<$T>() {
        //println!( "succeeded");
        unsafe {
          let raw: ::std::raw::TraitObject = ::std::mem::transmute (input);
          Some(::std::mem::transmute (raw.data))
        }
      }
      else {
        None
      }
    };
    result
  }}
}

#[doc (hidden)]
#[macro_export]
macro_rules! time_steward_common_impls_for_event_handle {
  ([$($bounds:tt)*] [$($concrete:tt)*] [$($basics:tt)*]) => {


    impl <$($bounds)*> Borrow<ExtendedTime <$($basics)*>> for $($concrete)* {
      fn borrow (&self)->& ExtendedTime <$($basics)*> {self.extended_time()}
    }

    /*impl<$($bounds)*> Eq for $($concrete)* {}
    impl<$($bounds)*> PartialEq for $($concrete)* {
      fn eq(&self, other: &Self) -> bool {
        self.extended_time().eq(other.extended_time())
      }
    }*/
    impl<$($bounds)*> Ord for $($concrete)* {
      fn cmp(&self, other: &Self) -> Ordering {
        self.extended_time().cmp(other.extended_time())
      }
    }
    impl<$($bounds)*> PartialOrd for $($concrete)* {
      fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
      }
    }


  };
}


#[doc (hidden)]
#[macro_export]
macro_rules! time_steward_common_impls_for_uniquely_identified_handle {
  ([$($bounds:tt)*] [$($concrete:tt)*] $self_hack: ident => ($id: expr): $Id: ty) => {

impl <$($bounds)*>  $($concrete)* {
  fn id (&$self_hack)->$Id {
    $id
  }
}
impl <$($bounds)*> Hash for $($concrete)* {
  fn hash <H: Hasher> (&self, state: &mut H) {
    self.id().hash (state);
  }
}
impl <$($bounds)*> Eq for $($concrete)* {}
impl <$($bounds)*> PartialEq for $($concrete)* {
  fn eq(&self, other: &Self) -> bool {
    self.id() == other.id()
  }
}

  };
}

#[doc (hidden)]
#[macro_export]
macro_rules! time_steward_common_impls_for_handles {
  () => {
    time_steward_common_impls_for_event_handle! ([B: Basics] [EventHandle <B>] [B]);
    time_steward_common_impls_for_uniquely_identified_handle! ([B: Basics] [EventHandle <B>] self => (self.extended_time().id): DeterministicRandomId);
    
    impl <T: StewardData + PersistentlyIdentifiedType> StewardData for DataHandle <T> {}
    impl <B: Basics> StewardData for EventHandle <B> {}
    impl <T: DataTimeline> StewardData for DataTimelineCell <T> {}
  };
}

/*pub trait DeserializationContext {
  fn deserialize_data <T: DeserializeOwned> (&mut self)->T;
  fn deserialize_timeline_handle <T: DataTimeline> (&mut self)->DataTimelineHandle <T>;
  fn deserialize_prediction_handle <T: Event> (&mut self)->PredictionHandle <T>;
  fn deserialize_event_handle <T: Event> (&mut self)->EventHandle <T>;
  fn deserialize_dynamic_event_handle (&mut self)->DynamicEventHandle;
  }*/


pub fn extended_time_of_fiat_event<B: Basics>(time: B::Time,
    id: DeterministicRandomId)
-> ExtendedTime<B> {
  ExtendedTime {
base: time,
      iteration: 0,
      id: id.for_fiat_event_internal(),
  }
}

  pub fn extended_time_of_predicted_event<B: Basics>
(event_base_time: B::Time,
 id: DeterministicRandomId,
 from: &ExtendedTime<B>)
  -> Option<ExtendedTime<B>> {
    let iteration = match event_base_time.cmp(&from.base) {
      Ordering::Less => return None, // short-circuit
      Ordering::Greater => 0,
      Ordering::Equal => {
        if id > from.id {
          from.iteration
        } else {
          if from.iteration >= B::MAX_ITERATION {
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
