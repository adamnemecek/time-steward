//! Serialization for TimeSteward data.
//!
//! TimeSteward has a few special requirements for serialization:
//! * DataTimelineHandle objects get special consideration, to support DAGs and cyclic data structures.
//! * The serialization needs to not block other operations for more than O(1) time at a time.
//! * The serialization must be lossless and platform-independent. For this reason, we always use bincode in low-endian mode.

macro_rules! time_steward_serialization_impls_for_handle {
  (
    [$($bounds:tt)*]
    [$($concrete:tt)*]
    (&$self_hack: ident)
    Uniquely identified by (($identifier: expr): $Identifier: ty)
    Data located at (| $handle: ident | &mut $data: expr)
  ) => {

/*thread_local! {
  static SERIALIZATION_CONTEXT: RefCell<Option <SerializationContext>> = RefCell::new (None);
  static DESERIALIZATION_CONTEXT: RefCell<Option <DeserializationContext>> = RefCell::new (None);
}*/


impl <$($bounds)*> $crate::serde::Serialize for $($concrete)* {
  fn serialize <S: $crate::serde::Serializer> (&$self_hack, serializer: S)->Result <S::Ok, S::Error> {
    unimplemented!()
    /*
    SERIALIZATION_CONTEXT.with (| cell | {
      let mut guard = cell.borrow_mut();
      let mut map = &mut guard.unwrap().map;
      map.insert (
    })
    $identifier.serialize (serializer)
    */
  }
}

impl <'a, $($bounds)*> $crate::serde::Deserialize <'a> for $($concrete)* {
  fn deserialize <D: $crate::serde::Deserializer<'a>> (deserializer: D)->Result <Self, D::Error> {
    unimplemented!()
    /*
    let old_identifier = $Identifier::deserialize (deserializer)?;
    DESERIALIZATION_CONTEXT.with (| cell | {
      let mut guard = cell.borrow_mut();
      let mut map = &mut guard.unwrap().map;
      Ok(map.entry (old_pointer).or_insert_with (| | {
        unimplemented!() 
        /*DataTimelineHandle {
          inner: Arc::new (unsafe {::std::mem::uninitialized()}),
        }*/
      }).clone().downcast::<T> ())
    })
    */
  }
}

/*
impl DeserializationContext {
  fn deserialize_(deserializer: D)->Result <(), D::Error> {
    let old_pointer = usize::deserialize (deserializer)?;
    DESERIALIZATION_CONTEXT.with (| cell | {
      let mut guard = cell.borrow_mut();
      let mut map = &mut guard.unwrap().map;
      let $handle = map.entry (old_pointer).or_insert_with (| | {
        unimplemented!()
        /*DataTimelineHandle {
          inner: Arc::new (unsafe {::std::mem::uninitialized()}),
        }*/
      });
      let new_pointer = &mut $data;
      unsafe {::std::ptr::write (
        new_pointer as *mut DataTimelineHandle,
        unimplemented!() //DataTimeline::deserialize (D)
      )?;}
    })?;
    Ok(())
  }
}*/


  };
}

macro_rules! time_steward_serialization_impls {
  () => {
  fn deserialize_something <R: Read> (reader: &mut R)->$crate::bincode::Result <()> {
    DESERIALIZATION_CONTEXT.with (| cell | {
      {
        let guard = cell.borrow_mut();
        assert!(guard.is_none(), "deserializing recursively breaks my hacks probably makes no sense");
        *guard = Some(DeserializationContext::new());
      }
      // deserialize inside a closure so that errors can be collected and we still clear the context afterwards
      let result = || {
        let time: ExtendedTime <B> = deserialize_from (reader, ::std::mem::size_of::<ExtendedTime <B>>())?;
        let global_timeline: DataTimelineHandle <B::GlobalTimeline> = deserialize_from (reader, $crate::bincode::Infinite)?;
    
        while !cell.borrow().uninitialized.is_empty() {
          let next: ????? = deserialize_from (reader, $crate::bincode::Infinite)?;
          match next {
            ?????::DataTimeline (type_id) => {
              let deserialize_function = cell.borrow().?????.get (type_id);
              (*deserialize_function)();
            }
            ?????::Event (type_id) => {
              let deserialize_function = cell.borrow().?????.get (type_id);
              (*deserialize_function)();
            }
          };
        }
      }();

      {
        let guard = cell.borrow_mut();
        *guard = None;
      }
      result
    })
  }
  };
}


