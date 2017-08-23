use std::mem;
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, Bound};
use std::cmp::{Ordering, max};
use std::borrow::Borrow;
use std::any::Any;
use std::io::{Read, Write};
use std::rc::Rc;
use std::marker::PhantomData;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use rand::Rng;

use super::super::api::*;
use super::super::implementation_support::common::*;
use implementation_support::common::split_off_greater_set;
use {DeterministicRandomId};

time_steward_steward_specific_api!();

thread_local! {
  static NEXT_SERIAL_NUMBER: Cell <usize> = Cell::new (0);
}
fn new_serial_number()->usize {
  NEXT_SERIAL_NUMBER.with (| cell | {
    let result = cell.get();
    cell.set (result + 1);
    result
  })
}

#[derive (Debug)]
pub struct DataTimelineCell <T: DataTimeline> {
  serial_number: usize,
  first_snapshot_not_updated: Cell<usize>,
  data: RefCell<T>,
}
#[derive (Debug)]
struct ExecutionState {
  valid: bool,
  execution_data: Box <Any>,
}
#[derive (Debug)]
struct EventInner <B: Basics> {
  time: ExtendedTime <B>,
  data: Box <EventInnerTrait<B>>,
  should_be_executed: Cell<bool>,
  prediction_created_by: RefCell<Option <EventHandle <B>>>,
  prediction_destroyed_by: RefCell<Option <EventHandle <B>>>,
  execution_state: RefCell<Option <ExecutionState>>,
}
trait EventInnerTrait <B: Basics>: Any + Debug {
  fn execute (&self, self_handle: & EventHandle <B>, steward: &mut Steward <B>);
  fn undo (&self, self_handle: & EventHandle <B>, steward: &mut Steward <B>);
  fn re_execute (&self, self_handle: & EventHandle <B>, steward: &mut Steward <B>);
}
impl <B: Basics, T: Event <Steward = Steward <B>>> EventInnerTrait <B> for T {
  fn execute (&self, self_handle: & EventHandle<B>, steward: &mut Steward <B>) {
    let mut accessor = EventAccessorStruct {
      generic: GenericEventAccessor::new(&self_handle.extended_time()),
      handle: self_handle.clone(),
      globals: steward.globals.clone(),
      steward: RefCell::new (steward),
    };
    let result = <T as Event>::execute (self, &mut accessor);
    mem::replace (&mut*self_handle.data.execution_state.borrow_mut(), Some (ExecutionState {
      valid: true,
      execution_data: Box::new (result),
    }));
  }
  fn undo (&self, self_handle: & EventHandle<B>, steward: &mut Steward <B>) {
    let mut accessor = EventAccessorStruct {
      generic: GenericEventAccessor::new(&self_handle.extended_time()),
      handle: self_handle.clone(),
      globals: steward.globals.clone(),
      steward: RefCell::new (steward),
    };
    <T as Event>::undo (self, &mut accessor, *self_handle.data.execution_state.borrow_mut().take().unwrap().execution_data.downcast().unwrap());
  }
  fn re_execute (&self, self_handle: & EventHandle<B>, steward: &mut Steward <B>) {
    let mut accessor = EventAccessorStruct {
      generic: GenericEventAccessor::new(&self_handle.extended_time()),
      handle: self_handle.clone(),
      globals: steward.globals.clone(),
      steward: RefCell::new (steward),
    };
    let result = <T as Event>::re_execute (self, &mut accessor, *self_handle.data.execution_state.borrow_mut().take().unwrap().execution_data.downcast().unwrap());
    mem::replace (&mut*self_handle.data.execution_state.borrow_mut(), Some (ExecutionState {
      valid: true,
      execution_data: Box::new (result),
    }));
  }
}


#[derive (Debug, Derivative)]
#[derivative (Clone (bound = ""))]
pub struct DataHandle <T: StewardData> {
  data: Rc<T>
}

#[derive (Debug, Derivative)]
#[derivative (Clone (bound = ""))]
pub struct EventHandle <B: Basics> {
  data: Rc <EventInner<B>>
}


impl <B: Basics> EventHandleTrait<B> for EventHandle <B> {
  fn extended_time (& self)->& ExtendedTime <B> {& self.data.time}
  fn downcast_ref <T: Any> (&self)->Option<&T> {
    downcast_ref!(&*self.data.data, T, EventInnerTrait<B>)
  }
}

impl <T: StewardData> DataHandleTrait <T> for DataHandle <T> {
  fn new(data: T)->Self {
    DataHandle { data: Rc::new(data) }
  }
}
impl <T: DataTimeline> DataTimelineCellTrait <T> for DataTimelineCell <T> {
  fn new(data: T)->Self {
    DataTimelineCell {
      serial_number: new_serial_number(),
      first_snapshot_not_updated: Cell::new (0),
      data: RefCell::new (data),
    }
  }
}
impl <T: DataTimeline> Clone for DataTimelineCell <T> {
  fn clone(&self)->Self {
    Self::new(self.data.borrow().clone())
  }
}

impl <T: StewardData> Deref for DataHandle <T> {
  type Target = T;
  fn deref (&self) -> &T {
    &*self.data
  }
}

time_steward_common_impls_for_handles!();
time_steward_common_impls_for_uniquely_identified_handle! ([B: Basics] [EventHandle <B>] self => (&*self.data as *const EventInner<B>): *const EventInner<B>);
time_steward_common_impls_for_uniquely_identified_handle! ([T: StewardData] [DataHandle <T>] self => (&*self.data as *const T): *const T);
time_steward_common_impls_for_uniquely_identified_handle! ([T: DataTimeline] [DataTimelineCell <T>] self => (self.serial_number): usize);

time_steward_serialization_impls_for_handle!(
  [T: DataTimeline] [DataTimelineCell <T>]
  (&self) Data located at (| handle | &mut handle.data)
);
time_steward_serialization_impls_for_handle!(
  [B: Basics] [EventHandle <B>]
  (&self) Data located at (| handle | &mut unimplemented!())
);
time_steward_serialization_impls_for_handle!(
  [T: StewardData] [DataHandle <T>]
  (&self) Data located at (| handle | &mut*handle.data)
);

#[derive (Debug)]
pub struct EventAccessorStruct <'a, B: Basics> {
  generic: GenericEventAccessor,
  handle: EventHandle <B>,
  globals: Rc<B::Globals>,
  steward: RefCell<&'a mut Steward<B>>,
}
#[derive (Debug)]
pub struct SnapshotInner <B: Basics> {
  index: usize,
  time: ExtendedTime <B>,
  globals: Rc<B::Globals>,
  clones: RefCell<HashMap<usize, Box<Any>>>,
  snapshots_tree: Rc<RefCell<SnapshotsTree<B>>>,
}
#[derive (Debug, Clone)]
pub struct SnapshotHandle <B: Basics> {
  data: Rc <SnapshotInner <B>>,
}

type SnapshotsTree<B> = BTreeMap<usize, SnapshotHandle <B>>;

impl <B: Basics> Drop for SnapshotHandle <B> {
  fn drop (&mut self) {
    assert!(Rc::strong_count(&self.data) >= 2);
    // if we are the last one dropped, our data still exists, and so does the entry in the tree
    if Rc::strong_count(&self.data) == 2 {
      // when we drop the one from the map recursively, that one will also observe a strong count of 2, so short-circuit it
      if let Ok (mut map) = self.data.snapshots_tree.try_borrow_mut() {
        map.remove (&self.data.index);
      }
    }
  }
}

impl <'a, B: Basics> Accessor for EventAccessorStruct <'a, B> {
  type Steward = Steward <B>;
  fn globals (&self)->&B::Globals {&*self.globals}
  fn extended_now(&self) -> & ExtendedTime <<Self::Steward as TimeSteward>::Basics> {
    self.handle().extended_time()
  }
  fn query <Query: StewardData, T: DataTimelineQueriableWith<Query, Basics = B>> (&self, timeline: & DataTimelineCell <T>, query: &Query, offset: QueryOffset)-> T::QueryResult {
    DataTimelineQueriableWith::<Query>::query (&*timeline.data.borrow(), query, self.extended_now(), offset)
  }
}
impl <B: Basics> Accessor for SnapshotHandle <B> {
  type Steward = Steward <B>;
  fn globals (&self)->&B::Globals {&*self.data.globals}
  fn extended_now(&self) -> & ExtendedTime <<Self::Steward as TimeSteward>::Basics> {
    & self.data.time
  }
  fn query <Query: StewardData, T: DataTimelineQueriableWith<Query, Basics = <Self::Steward as TimeSteward>::Basics>> (&self, timeline: & DataTimelineCell <T>, query: &Query, offset: QueryOffset)-> T::QueryResult {
    let mut guard = self.data.clones.borrow_mut();
    let entry = guard.entry (timeline.serial_number);
    let boxref = entry.or_insert_with (| | Box::new (
      timeline.data.borrow().clone_for_snapshot (self.extended_now())
    ));
    let typed = boxref.downcast_ref::<T>().unwrap();
    DataTimelineQueriableWith::<Query>::query(typed, query, self.extended_now(), offset)
  }
}
impl <'a, B: Basics> EventAccessor for EventAccessorStruct <'a, B> {
  fn handle (&self)->& EventHandle <B> {
    &self.handle
  }
  
  fn modify <T: DataTimeline<Basics = <Self::Steward as TimeSteward>::Basics>, F: FnOnce(&mut T)> (&self, timeline: &DataTimelineCell <T>, modification: F) {
    let index = timeline.first_snapshot_not_updated.get ();
    let steward = self.steward.borrow();
    let guard = (*steward.snapshots).borrow();
    let map: &SnapshotsTree<B> = &*guard;
    for (_,snapshot) in map.range ((Bound::Included(index), Bound::Unbounded)) {
      let mut guard = snapshot.data.clones.borrow_mut();
      let entry = guard.entry (timeline.serial_number);
      entry.or_insert_with (| | Box::new (timeline.data.borrow().clone_for_snapshot (self.extended_now())));
    }
    timeline.first_snapshot_not_updated.set (steward.next_snapshot_index);
    
    let mut modify_guard = timeline.data.borrow_mut();
    modification (&mut*modify_guard);
    match &steward.invalid_before {
      &ValidSince::Before (ref time) => modify_guard.forget_before(&ExtendedTime::beginning_of (time.clone())),
      &ValidSince::After (ref time) => modify_guard.forget_before(&ExtendedTime::end_of(time.clone())),
      &ValidSince::TheBeginning => (),
    }
  }
  
  fn create_prediction <E: Event <Steward = Self::Steward>> (&self, time: <<Self::Steward as TimeSteward>::Basics as Basics>::Time, id: DeterministicRandomId, event: E)->EventHandle <B> {
    let time = extended_time_of_predicted_event::<<Self::Steward as TimeSteward>::Basics> (time, id, self.extended_now()).expect("You can't create a prediction in the past.");
    let handle = EventHandle {
      data: Rc::new (EventInner {
        time: time,
        data: Box::new (event),
        should_be_executed: Cell::new(true),
        prediction_created_by: RefCell::new (Some(self.handle().clone())),
        prediction_destroyed_by: RefCell::new (None),
        execution_state: RefCell::new (None),
      })
    };
    assert!(self.steward.borrow_mut().events_needing_attention.insert (handle.clone()), "created a prediction that already existed?!");
    handle
  }
  fn destroy_prediction (&self, prediction: &EventHandle<B>) {
    assert!(prediction.data.prediction_created_by.borrow().is_some(), "Attempted to destroy a fiat event as if it was a prediction.");
    let mut guard = prediction.data.prediction_destroyed_by.borrow_mut();
    if let Some (old_destroyer) = guard.as_ref() {
      assert!(self.handle() < old_destroyer, "You can't destroy a prediction that was already destroyed. (A prediction is supposed to be destroyed exactly when it's no longer accessible in the simulation data. Double-destroying it implies that you held onto a handle to it somewhere, which is probably a bug.)");
    }
    mem::replace (&mut*guard, Some(self.handle().clone()));
    if prediction != self.handle() {
      self.steward.borrow_mut().event_shouldnt_be_executed (prediction);
    }
  }
  
  fn invalidate <I: Invalidator <Steward = Self::Steward>> (&self, invalidator: I) {
    invalidator.execute(self);
  }
}
impl <'a, B: Basics> Rng for EventAccessorStruct <'a, B> {
  fn next_u32(&mut self) -> u32 {self.generic.generator.next_u32()}
    fn next_f32(&mut self) -> f32 {
      panic!("Using floating point numbers in TimeSteward events is forbidden because it is nondeterministic across platforms.")
    }
    fn next_f64(&mut self) -> f64 {
      panic!("Using floating point numbers in TimeSteward events is forbidden because it is nondeterministic across platforms.")
    }
}

impl <B: Basics> SnapshotAccessor for SnapshotHandle <B> {
  fn serialize_into <W: Write> (&self, writer: W) {
    unimplemented!()
  }
}

// EventAccessorStruct is also the undo accessor and invalidation accessor – its functionality is only restricted by what bounds the client code is allowed to place on it

impl <'a, B: Basics> PeekingAccessor for EventAccessorStruct <'a, B> {
  fn peek <T: DataTimeline<Basics = <Self::Steward as TimeSteward>::Basics>, R, F: FnOnce(&T)->R> (&self, timeline: & DataTimelineCell<T>, callback: F)->R {
    callback(&*timeline.data.borrow())
  }
}
impl <'a, B: Basics> UndoEventAccessor for EventAccessorStruct <'a, B> {
  fn undestroy_prediction (&self, prediction: &<Self::Steward as TimeSteward>::EventHandle, until: Option <&<Self::Steward as TimeSteward>::EventHandle>) {
    mem::replace (&mut*prediction.data.prediction_destroyed_by.borrow_mut(), until.cloned());
    if prediction != self.handle() {
      if !prediction.data.should_be_executed.get() && prediction.data.execution_state.borrow().as_ref().map_or (true, | state | !state.valid) {
        self.steward.borrow_mut().events_needing_attention.insert (prediction.clone());
      }
      prediction.data.should_be_executed.set(true);
    }
  }
}
impl <'a, B: Basics> InvalidationAccessor for EventAccessorStruct <'a, B> {
  fn invalidate (&self, handle: & <Self::Steward as TimeSteward>::EventHandle) {
    assert!(handle > self.handle(), "Only future events can be invalidated.");
    self.steward.borrow_mut().invalidate_event_execution (handle);
  }
}


#[derive (Debug)]
pub struct Steward <B: Basics> {
  globals: Rc<B::Globals>,
  invalid_before: ValidSince <B::Time>,
  events_needing_attention: BTreeSet<EventHandle<B>>,
  fiat_events: BTreeSet<EventHandle <B>>,
  snapshots: Rc<RefCell<SnapshotsTree<B>>>,
  next_snapshot_index: usize,
}


impl<B: Basics> Steward<B> {
  fn next_event_needing_attention (&self) -> Option<&EventHandle<B>> {
    self.events_needing_attention.iter().next()
  }
  
  fn do_event (&mut self, event: & EventHandle <B>) {
    self.events_needing_attention.remove (event);
    if event.data.should_be_executed.get() {
      let currently_executed = match event.data.execution_state.borrow().as_ref() {
        Some (state) => {
          assert! (!state.valid);
          true
        },
        None => false,
      };
      if currently_executed {
        event.data.data.re_execute (event, &mut*self);
      }
      else {
        event.data.data.execute (event, &mut*self);
      }
      if event.data.prediction_created_by.borrow().is_some() {
        assert!(event.data.prediction_destroyed_by.borrow().as_ref() == Some(event), "All predicted events must destroy the prediction that predicted them. (It's ambiguous what should happen if the prediction isn't destroyed. There are two natural meanings: either it continues existing meaninglessly, or it gets executed repeatedly until it destroys itself. Neither of these seems especially desirable, so we take the conservative approach and forbidden the whole situation from arising. This is also future-proofing in case we choose a specific behavior later.")
      }
    }
    else {
      assert! (event.data.execution_state.borrow().as_ref().map_or (false, | state | !state.valid));
      event.data.data.undo (event, &mut*self);
    }
  }
  
  
  
  fn invalidate_event_execution (&mut self, handle: & EventHandle<B>) {
    if let Some(state) = handle.data.execution_state.borrow_mut().as_mut() {
      if state.valid {self.events_needing_attention.insert (handle.clone());}
      state.valid = false;
    }
  }
  fn event_shouldnt_be_executed (&mut self, handle: & EventHandle<B>) {
    if handle.data.should_be_executed.get() {
      if handle.data.execution_state.borrow().as_ref().map_or (false, | state | state.valid) {
        self.events_needing_attention.insert (handle.clone());
      }
      if handle.data.execution_state.borrow().is_none() {
        self.events_needing_attention.remove (handle);
      }
    }
    handle.data.should_be_executed.set(false);
  }
}


impl<B: Basics> TimeSteward for Steward<B> {
  type Basics = B;
  type SnapshotAccessor = SnapshotHandle <B>;
  type EventHandle = EventHandle <B>;

  fn valid_since(&self) -> ValidSince<B::Time> {
    self.invalid_before.clone()
  }
  
  fn insert_fiat_event<E: Event<Steward = Self>>(&mut self,
                                               time: B::Time,
                                               id: DeterministicRandomId,
                                               event: E)
                                               -> Result<(), FiatEventOperationError> {
    if self.valid_since() > time {
      return Err(FiatEventOperationError::InvalidTime);
    }
    let handle = EventHandle {data: Rc::new (EventInner {
        time: extended_time_of_fiat_event(time, id),
        data: Box::new (event),
        should_be_executed: Cell::new(true),
        prediction_created_by: RefCell::new (None),
        prediction_destroyed_by: RefCell::new (None),
        execution_state: RefCell::new (None),
      })};
    match self.fiat_events.insert(handle.clone()) {
      false => Err(FiatEventOperationError::InvalidInput),
      true => {
        self.events_needing_attention.insert (handle);
        Ok(())
      },
    }
  }

  fn remove_fiat_event(&mut self,
                       time: &B::Time,
                       id: DeterministicRandomId)
                       -> Result<(), FiatEventOperationError> {
    if self.valid_since() > *time {
      return Err(FiatEventOperationError::InvalidTime);
    }
    match self.fiat_events.take(&extended_time_of_fiat_event(time.clone(), id)) {
      None => Err(FiatEventOperationError::InvalidInput),
      Some(handle) => {
        self.event_shouldnt_be_executed (&handle);
        Ok(())
      },
    }
  }
  
  fn snapshot_before (&mut self, time: & B::Time)->Option <Self::SnapshotAccessor> {
    // NOT self.valid_since(); this Steward can continue recording snapshots from earlier than the earliest time it can accept fiat event input
    if self.invalid_before > *time { return None; }
    while let Some (updated) = self.updated_until_before () {
      if updated >= *time {break;}
      self.step();
    }
    let handle = SnapshotHandle {
      data: Rc::new (SnapshotInner {
        index: self.next_snapshot_index,
        globals: self.globals.clone(),
        time: ExtendedTime::beginning_of(time.clone()),
        clones: RefCell::new (HashMap::new()),
        snapshots_tree: self.snapshots.clone(),
      })
    };
    self.snapshots.borrow_mut().insert (self.next_snapshot_index, handle.clone());
    self.next_snapshot_index += 1;
    Some (handle)
  }
  
  fn forget_before (&mut self, time: & B::Time) {
    self.invalid_before = max (self.invalid_before.clone(), ValidSince::Before(time.clone()));
    
  }
}


impl <B: Basics> ConstructibleTimeSteward for Steward <B> {
  fn from_globals (globals: <Self::Basics as Basics>::Globals)->Self {
    Steward {
      globals: Rc::new (globals),
      invalid_before: ValidSince::TheBeginning,
      events_needing_attention: BTreeSet::new(),
      fiat_events: BTreeSet::new(),
      snapshots: Rc::new (RefCell::new (BTreeMap::new())),
      next_snapshot_index: 0,
    }
  }
  
  fn deserialize_from <R: Read> (data: &mut R)->Self {
    unimplemented!()
  }
}

impl<B: Basics> IncrementalTimeSteward for Steward<B> {
  fn step(&mut self) {
    if let Some(event) = self.next_event_needing_attention().cloned() {
      self.do_event(&event);
    }
  }
  fn updated_until_before(&self) -> Option<B::Time> {
    self.next_event_needing_attention().map(|event| event.extended_time().base.clone())
  }
}
impl<B: Basics> CanonicalTimeSteward for Steward<B> {}

time_steward_define_simple_timeline!();