use ::DeterministicRandomId;
use std::hash::Hash;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::io::{Read, Write};
use std::any::Any;
use std::fmt::{self, Debug};

/// Data used for a TimeSteward simulation, such as times, entities, and events.
///
/// TimeSteward has strong requirements for serializability. In addition to implementing these traits, a StewardData must have Eq be equivalent to equality of its serialization, and all its behavior must be invariant under cloning and serialize-deserialize cycles.
pub trait StewardData: Any + Send + Sync + Clone + Eq + Serialize + DeserializeOwned + Debug {}


// Model: events interact with the physics only through queries at their exact time (which are forbidden to query other timelines or have any side effects) and modifications at their exact time (which are forbidden to return any information). Those modifications, in practice, change the state *going forward from* that time, and the events must use invalidate() to collect future events that must be invalidated. (Although, for instance, modifications made for dependency tracking purposes don't change any results of queries, so they never need to invalidate anything themselves.)
//To audit, we record all of the queries, query results, and modifications. Then after each event that modifies one or more DataTimeline, we rerun all queries to those timelines made by still-valid future events. If any query has a different result than before, it's an error.

// Given a query input, the function (time->query output) must be piecewise constant, changing only at times when modifications have been inserted.
// Event must implement undo. That must call the undo function matching all modifications it made. After an event is undone, there may not remain any modifications at the time of that event. It follows that after doing and then undoing an event, all queries IMMEDIATELY after the event time are restored to their former value. However, queries in the future might not be restored because modifications are permitted to mess up the future(?)

// Defined by each TimeSteward type; would be associated type constructors if Rust supported those; temporarily defining them here to allow the API to compile by itself
pub struct DataTimelineHandle <T: DataTimeline> {t:T}
pub struct EventHandle <T: Event> {t:T}
pub struct DynamicEventHandle <B: Basics> {b:B}
pub struct PredictionHandle <T: Event> {t:T}
impl <T: Event> EventHandle <T> {
  pub fn erase_type (self)->DynamicEventHandle<<T::Steward as TimeSteward>::Basics> {unimplemented!()}
}
impl <T: Event> EventHandleTrait for EventHandle <T> {
  type Basics = <T::Steward as TimeSteward>::Basics;
  fn time (&self)->& ExtendedTime <Self::Basics> {unimplemented!()}
}
impl <B: Basics> EventHandleTrait for DynamicEventHandle<B> {
  type Basics = B;
  fn time (&self)->& ExtendedTime <Self::Basics> {unimplemented!()}
}

pub enum QueryOffset {
  Before, After
}

pub trait DataTimeline: Any + Serialize + DeserializeOwned {
  type Basics: Basics;

  /// Make a clone of only the data necessary to report accurately at a specific time.
  // audit: the clone yields the same query results immediately before and after the time
  fn clone_for_snapshot (&self, time: &ExtendedTime <Self::Basics>)->Self;
  
  // audit: forget functions don't change any query results except those forgotten
  fn forget_before (&mut self, time: &ExtendedTime <Self::Basics>);
}
pub trait DataTimelineQueriableWith<Query: StewardData>: DataTimeline {
  type QueryResult: StewardData;
  
  // audit all functions: must be consistent with each other
  // audit: queries must not have side effects (do a separate action for manual dependency tracking)
  // audit: queries don't return PredictionHandles that don't exist at the time
  // TODO: allow queries to return references instead of values
  fn query (&self, query: &Query, time: &ExtendedTime <Self::Basics>, offset: QueryOffset)->Self::QueryResult;
  // TODO: is this necessary? Or is it only used in invalidation code, which can peek anyway?
  // fn query_range (&self, query: Query, time_range: TimeRange)->impl Iter <Item = (TimeRange, QueryResult)>;
}
pub trait Event: StewardData {
  type Steward: TimeSteward;
  // audit all functions: calls invalidate_event for everything whose queries would be changed
  // audit all functions: doesn't change any query results in the past
  fn execute<Accessor: EventAccessor <Steward = Self::Steward, Event = Self>> (&mut self, accessor: &Accessor);
  // audit: leaves self in its original state??
  // audit: after undoing, all query results immediately before and after the event are identical to each other (if the previous audit passes, this is more an audit of the DataTimeline types than this event type)
  fn undo<Accessor: UndoEventAccessor <Steward = Self::Steward, Event = Self>> (&mut self, accessor: &Accessor);
  // audit: should produce the same subsequent query results as doing undo() and then execute()
  // implementing this is simply an optimization that may allow you to invalidate fewer things, so we default-implement it
  fn re_execute<Accessor: UndoEventAccessor <Steward = Self::Steward, Event = Self>> (&mut self, accessor: &Accessor) {
    self.undo (accessor);
    self.execute (accessor);
  }
}


/**
This is intended to be implemented on an empty struct. Requiring Clone etc. is a hack to work around [a compiler weakness](https://github.com/rust-lang/rust/issues/26925).
*/
pub trait Basics
  : Any + Send + Sync + Copy + Clone + Ord + Hash + Serialize + DeserializeOwned + Debug + Default {
  type Time: StewardData + Ord + Hash;
  type GlobalTimeline: DataTimeline;
  const MAX_ITERATION: IterationType = 65535;
}

pub type IterationType = u32;
#[derive (Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct ExtendedTime<B: Basics> {
  pub base: B::Time,
  pub iteration: IterationType,
  pub id: DeterministicRandomId,
}

#[derive (Copy, Clone, PartialEq, Eq, Debug)]
pub enum FiatEventOperationError {
  InvalidInput,
  InvalidTime,
}

pub trait EventHandleTrait {
  type Basics: Basics;
  fn time (&self)->& ExtendedTime <Self::Basics>;
}

/*pub struct TimeRange <Time> {
  pub start: Time,
  pub end: Option <Time>,
}*/

// This exists to support a variety of time stewards
// along with allowing BaseTime to be dense (e.g. a
// rational number rather than an integer).
// It is an acceptable peculiarity that even for integer times,
// After(2) < Before(3).
// #[derive (Copy, Clone, PartialEq, Eq, Hash)]
#[derive (Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub enum ValidSince<BaseTime> {
  TheBeginning,
  Before(BaseTime),
  After(BaseTime),
}



pub trait Accessor {
  type Steward: TimeSteward;
  fn global_timeline (&self)->DataTimelineHandle <<<Self::Steward as TimeSteward>::Basics as Basics>::GlobalTimeline>;
  fn query <Query: StewardData, T: DataTimelineQueriableWith<Query, Basics = <Self::Steward as TimeSteward>::Basics>> (&self, handle: & DataTimelineHandle <T>, query: &Query, offset: QueryOffset)-> T::QueryResult;
}
// Querying versus peeking:
// Querying accessors are generally for things that can affect the physics. Querying uses an exact interface that can be tracked and audited in various ways to make sure the physics stays consistent.
// Peeking accessors are generally for things that are required to do a specific job and don't have any leeway to change the physics. We allow them full read-only access with no tracking, and merely audit that they did the job they were asked to. Peeking accessors are also allowed to use the querying interface for convenience (so that they can call generic functions that take a querying accessor).
pub trait PeekingAccessor: Accessor {
  fn peek <T: DataTimeline<Basics = <Self::Steward as TimeSteward>::Basics>> (&self, handle: & DataTimelineHandle <T>)->& T;
}
pub trait MomentaryAccessor: Accessor {
  fn now(&self) -> & ExtendedTime <<Self::Steward as TimeSteward>::Basics>;
}
pub trait EventAccessor: MomentaryAccessor {
  type Event: Event <Steward = Self::Steward>;
  fn handle (&self)->& EventHandle <Self::Event>;
  
  // modification is done within a Fn so that the event can't extract any information from DataTimelines except by querying.
  fn modify <T: DataTimeline<Basics = <Self::Steward as TimeSteward>::Basics>, F: Fn(&mut T)> (&self, timeline: &DataTimelineHandle <T>, modification: &F);
  
  // audit: whenever an event is executed or undone, it creates/destroys the exact predictions that become existent/nonexistent between the serializations of the physics immediately before and after the event.
  // audit: never generates two predictions with the same id, except when rerunning the same event
  fn create_prediction <E: Event <Steward = Self::Steward>> (&self, time: <<Self::Steward as TimeSteward>::Basics as Basics>::Time, id: DeterministicRandomId, event: E)->PredictionHandle<E>;
  fn destroy_prediction <E: Event <Steward = Self::Steward>> (&self, prediction: &PredictionHandle<E>);
  
  // invalidation is done within a Fn so that the event can't extract any information from the PeekingAccessor used for invalidation.
  fn invalidate <A: InvalidationAccessor, F: Fn(&A)> (&self, invalidator: &F);
}
pub trait UndoEventAccessor: PeekingAccessor + EventAccessor {
  // note that query results wouldn't necessarily correspond to those observed by the original execution in any way
  fn undestroy_prediction <E: Event <Steward = Self::Steward>> (&self, prediction: &PredictionHandle<E>, until: Option <&ExtendedTime <<Self::Steward as TimeSteward>::Basics>>);
}
pub trait InvalidationAccessor: PeekingAccessor {
  // if you use queries, note that there may be multiple relevant times and this might be in an undo (see above)
  // audit: can't invalidate things in the past relative to the current event
  fn invalidate <T: Event <Steward = Self::Steward>> (&self, handle: & EventHandle <T>);
  fn invalidate_dynamic (&self, handle: & DynamicEventHandle<<Self::Steward as TimeSteward>::Basics>);
}

pub trait Snapshot: MomentaryAccessor {
  /// note: Snapshot::serialize() matches TimeSteward::deserialize()
  fn serialize_into <W: Write> (&self, writer: W);
}

impl <T: EventAccessor> MomentaryAccessor for T {
  fn now(&self) -> & ExtendedTime <<Self::Steward as TimeSteward>::Basics> {
    &self.handle().time()
  }
}


/*pub trait DeserializationContext {
  fn deserialize_data <T: DeserializeOwned> (&mut self)->T;
  fn deserialize_timeline_handle <T: DataTimeline> (&mut self)->DataTimelineHandle <T>;
  fn deserialize_prediction_handle <T: Event> (&mut self)->PredictionHandle <T>;
  fn deserialize_event_handle <T: Event> (&mut self)->EventHandle <T>;
  fn deserialize_dynamic_event_handle (&mut self)->DynamicEventHandle;
}*/

pub trait TimeSteward: Any {
  type Basics: Basics;
  type Snapshot: Snapshot;
  
  fn from_global_timeline (timeline: <Self::Basics as Basics>::GlobalTimeline)->Self;
  /// note: Snapshot::serialize() matches TimeSteward::deserialize()
  fn deserialize_from <R: Read> (data: &mut R)->Self;
  
  fn insert_fiat_event<E: Event>(&mut self, time: <Self::Basics as Basics>::Time, id: DeterministicRandomId, event: E)
                                               -> Result<(), FiatEventOperationError>;
  fn remove_fiat_event(&mut self, time: &<Self::Basics as Basics>::Time, id: DeterministicRandomId)
                       -> Result<(), FiatEventOperationError>;
  fn snapshot_before (&mut self, time: &<Self::Basics as Basics>::Time)->Option <Self::Snapshot>;
  
  fn valid_since(&self) -> ValidSince<<Self::Basics as Basics>::Time>;
  fn forget_before (&mut self, time: &<Self::Basics as Basics>::Time);
}

