//use time_steward::support;
use time_steward::support::time_functions::QuadraticTrajectory;
use nalgebra::Vector2;
//use time_steward::support::rounding_error_tolerant_math::right_shift_round_up;


use time_steward::{DeterministicRandomId};
use time_steward::{PersistentTypeId, ListedType, PersistentlyIdentifiedType, DataHandleTrait, DataTimelineCellTrait, Basics as BasicsTrait};
pub use time_steward::stewards::{simple_full as steward_module};
use steward_module::{TimeSteward, Event, DataHandle, DataTimelineCell, Accessor, EventAccessor, FutureCleanupAccessor, bbox_collision_detector as collisions};
use simple_timeline::{SimpleTimeline, tracking_query, tracking_query_ref, set};

use rand::Rng;

pub type Time = i64;
pub type SpaceCoordinate = i64;


pub const HOW_MANY_CIRCLES: usize = 20;
pub const ARENA_SIZE_SHIFT: u32 = 20;
pub const ARENA_SIZE: SpaceCoordinate = 1 << 20;
pub const GRID_SIZE_SHIFT: u32 = ARENA_SIZE_SHIFT - 3;
// pub const GRID_SIZE: SpaceCoordinate = 1 << GRID_SIZE_SHIFT;
pub const MAX_DISTANCE_TRAVELED_AT_ONCE: SpaceCoordinate = ARENA_SIZE << 4;
pub const TIME_SHIFT: u32 = 20;
pub const SECOND: Time = 1 << TIME_SHIFT;

#[derive (Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Debug, Default)]
pub struct Basics {}
impl BasicsTrait for Basics {
  type Time = Time;
  type Globals = Globals;
  type Types = (ListedType <RelationshipChange>, ListedType <BoundaryChange>, ListedType <Initialize>, ListedType <Disturb>, collisions::simple_grid::Types <Space>);
}

pub type Steward = steward_module::Steward <Basics>;


pub struct Globals {
  circles: Vec<CircleHandle>,
  detector: DataTimelineCell <SimpleTimeline <DataHandle <SimpleGridDetector<Space>>, Steward>>,
}

#[derive (Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct Circle {
  pub index: usize,
  pub radius: SpaceCoordinate,
  pub varying: DataTimelineCell <SimpleTimeline <CircleVarying, Steward>>,
}
#[derive (Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct CircleVarying {
  pub position: QuadraticTrajectory,
  pub last_change: Time,
  pub relationships: Vec<RelationshipHandle>,
  pub boundary_induced_acceleration: Option <Vector2<SpaceCoordinate>>,
  pub next_boundary_change: Option <<Steward as TimeSteward>::EventHandle>,
}
impl PersistentlyIdentifiedType for Circle {
  const ID: PersistentTypeId = PersistentTypeId(0xd711cc7240c71607);
}
type CircleHandle = DataHandle <Circle>;

#[derive (Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct Relationship {
  pub circles: (CircleHandle, CircleHandle),
  pub varying: DataTimelineCell <SimpleTimeline <RelationshipVarying, Steward>>,
}
#[derive (Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct RelationshipVarying {
  pub induced_acceleration: Option <Vector2<SpaceCoordinate>>,
  pub next_change: Option <<Steward as TimeSteward>::EventHandle>,
}
impl PersistentlyIdentifiedType for Relationship {
  const ID: PersistentTypeId = PersistentTypeId(0xa1010b5e80c3465a);
}
type RelationshipHandle = DataHandle <Relationship>;


#[derive (Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
struct Space;
impl PersistentlyIdentifiedType for Space {
  const ID: PersistentTypeId = PersistentTypeId(0x879511343e48addd);
}
impl collisions::Space for Space {
  type Steward = Steward;
  type Object = Circle;
  type DetectorDataPerObject = collisions::simple_grid::DetectorDataPerObject<Self>;
  
  const DIMENSIONS: NumDimensions = 2;

  // An Object generally has to store some opaque data for the collision detector.
  // It would normally include a DataHandle to a tree node.
  // These are getter and setter methods for that data.
  fn get_detector_data<A: EventAccessor <Steward = Self::Steward>>(&self, accessor: &A, object: &DataHandle<Self::Object>)->Option<&DetectorDataPerObject<Self>>> {Some(query (accessor, object.varying).collision_data)}
  fn set_detector_data<A: EventAccessor <Steward = Self::Steward>>(&self, accessor: &A, object: &DataHandle<Self::Object>, data: Option<DetectorDataPerObject<Self>>);

  fn current_bounding_box<A: EventAccessor <Steward = Self::Steward>>(&self, accessor: &A, object: &DataHandle<Self::Object>)->BoundingBox {
    let varying = tracking_query (accessor, & object.varying);
    let center = varying.position.updated_by (accessor.now() - varying.last_change).unwrap().evaluate();
    BoundingBox {bounds: [
      [center [0] - object.radius, center [0] + object.radius],
      [center [1] - object.radius, center [1] + object.radius],
    ]}
  }
  fn when_escapes<A: EventAccessor <Steward = Self::Steward>>(&self, accessor: &A, object: &DataHandle<Self::Object>, space: &Self::Space, BoundingBox)-><Self::Steward as TimeSteward>::Basics::Time {
    let varying = tracking_query (accessor, & object.varying);
    varying.position.approximately_when_escapes (
      varying.last_change.clone(),
      accessor.now().clone(),
      [
        [bounds [0] [0] + object.radius, bounds [0] [1] - object.radius],
        [bounds [1] [0] + object.radius, bounds [1] [1] - object.radius],
      ]
    )
  }
  
  fn become_neighbors<A: EventAccessor <Steward = Self::Steward>>(&self, accessor: &A, objects: [&DataHandle<Self::Object>; 2]) {
    let relationship = accessor.new_handle (Relationship {
      circles: (objects [0].clone(), objects [1].clone()),
      varying: DataTimelineCell::new(SimpleTimeline::new ()),
    });
    set (accessor, & relationship.varying, RelationshipVarying {
      induced_acceleration: None,
      next_change: None,
    });
    for object in objects.iter() {
      let mut varying = tracking_query (accessor, & object.varying);
      varying.relationships.push (relationship.clone());
      set (accessor, & object.varying, varying);
    }
    update_relationship_change_prediction (accessor, relationship) ;
  }
  fn stop_being_neighbors<A: EventAccessor <Steward = Self::Steward>>(&self, accessor: &A, objects: [&DataHandle<Self::Object>; 2]) {
    let varying = tracking_query (accessor, & objects[0].varying);
    let relationship = varying.relationships.iter().find (| relationship | (
      relationship.circles == (objects[0], objects[1])
      || relationship.circles == (objects[1], objects[0])
    )).unwrap().clone();
    destroy (accessor, & relationship.varying);
    for object in objects.iter() {
      let mut varying = tracking_query (accessor, & object.varying);
      varying.relationships.retain (| relationship | !(
        relationship.circles == (objects[0], objects[1])
        || relationship.circles == (objects[1], objects[0])
      ));
      set (accessor, & object.varying, varying);
    }
  }
}





pub fn update_relationship_change_prediction <Accessor: EventAccessor <Steward = Steward>>(accessor: &Accessor, relationship_handle: &RelationshipHandle) {
  let circles = &relationship_handle.circles;
  let now = accessor.extended_now().clone();
  let mut relationship_varying = tracking_query (accessor, & relationship_handle.varying);
  let us = (
    tracking_query_ref (accessor, & circles.0.varying),
    tracking_query_ref (accessor, & circles.1.varying));

  let time = QuadraticTrajectory::approximately_when_distance_passes(circles.0.radius +
                                                                   circles.1.radius,
                                                                   if relationship_varying.induced_acceleration.is_none() {
                                                                     -1
                                                                   } else {
                                                                     1
                                                                   },
                                                                   ((us.0).last_change,
                                                                    &(us.0).position),
                                                                   ((us.1).last_change,
                                                                    &(us.1).position));
  // println!("Planning for {} At {}, {}", id, (us.0).1, (us.1).1);
  if time.is_none() && relationship_varying.induced_acceleration.is_some() {
    panic!(" fail {:?} {:?} {:?}", relationship_handle, relationship_varying, us)
  }
  
  relationship_varying.next_change = None;
  if let Some(yes) = time {
    if yes >= *accessor.now() {
      // println!(" planned for {}", &yes);
      relationship_varying.next_change = Some(accessor.create_prediction (
        yes,
        DeterministicRandomId::new (&(now.id, circles.0.index, circles.1.index.wrapping_add (0x6515c48170b61837))),
        RelationshipChange {relationship_handle: relationship_handle.clone()}
      ));
    }
  }
  set (accessor, & relationship_handle.varying, relationship_varying);
}

#[derive (Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct RelationshipChange {pub relationship_handle: RelationshipHandle} //, Basics, EventId (0x2312e29e341a2495),
impl PersistentlyIdentifiedType for RelationshipChange {
  const ID: PersistentTypeId = PersistentTypeId(0x08c4b60ad5d0ed08);
}
impl Event for RelationshipChange {
  type Steward = Steward;
  type ExecutionData = ();
  fn execute <Accessor: EventAccessor <Steward = Self::Steward>> (&self, accessor: &mut Accessor) {
    let circles = &self.relationship_handle.circles;
    let mut relationship_varying = tracking_query (accessor, &self.relationship_handle.varying);
    let mut new = (
      tracking_query (accessor, & circles.0.varying),
      tracking_query (accessor, & circles.1.varying));
    new.0.position.update_by(accessor.now() - new.0.last_change);
    new.1.position.update_by(accessor.now() - new.1.last_change);
    new.0.last_change = accessor.now().clone();
    new.1.last_change = accessor.now().clone();
    //let new_difference = new.0.position.evaluate ()-new.1.position.evaluate ();
    //println!("event with error {:?}", (new_difference.dot(&new_difference) as f64).sqrt() - (circles.0.radius+circles.1.radius)  as f64);
    if let Some(induced_acceleration) = relationship_varying.induced_acceleration {
      new.0
        .position
        .add_acceleration(-induced_acceleration);
      new.1
        .position
        .add_acceleration(induced_acceleration);
      relationship_varying.induced_acceleration = None;
      //println!("Parted {} At {}", self.id, mutator.now());
    } else {
      let acceleration = (new.0.position.evaluate() -
                          new.1.position.evaluate()) *
                          (ARENA_SIZE * 4 /
                           (circles.0.radius + circles.1.radius));
      new.0.position.add_acceleration(acceleration);
      new.1.position.add_acceleration(-acceleration);
      relationship_varying.induced_acceleration = Some(acceleration);
        //println!("Joined {} At {}", self.id, mutator.now());
    }
    set (accessor, & self.relationship_handle.varying, relationship_varying);
    set (accessor, & circles.0.varying, new.0.clone());
    set (accessor, & circles.1.varying, new.1.clone());
    SimpleGridDetector::changed_course(accessor, accessor.globals().detector, & circles.0);
    SimpleGridDetector::changed_course(accessor, accessor.globals().detector, & circles.1);
    // TODO no repeating the relationship between these 2 in particular
    update_predictions (accessor, &circles.0, & new.0);
    update_predictions (accessor, &circles.1, & new.1);
  }

  fn undo <Accessor: FutureCleanupAccessor <Steward = Self::Steward>> (&self, accessor: &mut Accessor, _: ()) {
    unimplemented!()
  }
}

pub fn update_boundary_change_prediction <Accessor: EventAccessor <Steward = Steward>>(accessor: &Accessor, circle_handle: &CircleHandle) {
  let arena_center = QuadraticTrajectory::new(TIME_SHIFT,
                                              MAX_DISTANCE_TRAVELED_AT_ONCE,
                                              [ARENA_SIZE / 2, ARENA_SIZE / 2, 0, 0, 0, 0]);
  let now = accessor.extended_now().clone();
  let mut varying = tracking_query (accessor, & circle_handle.varying);

  let time = QuadraticTrajectory::approximately_when_distance_passes(ARENA_SIZE - circle_handle.radius,
                                                                   if varying.boundary_induced_acceleration.is_some() {
                                                                     -1
                                                                   } else {
                                                                     1
                                                                   },
                                                                   (varying.last_change,
                                                                    & varying.position),
                                                                    (0, & arena_center));
  
  varying.next_boundary_change = None;
  if let Some(yes) = time {
    if yes >= *accessor.now() {
      // println!(" planned for {}", &yes);
      varying.next_boundary_change = Some(accessor.create_prediction (
        yes,
        DeterministicRandomId::new (&(now.id, circle_handle.index)),
        BoundaryChange {circle_handle: circle_handle.clone()}
      ));
    }
  }
  set (accessor, & circle_handle.varying, varying);
}

pub fn update_predictions <Accessor: EventAccessor <Steward = Steward>>(accessor: &Accessor, circle_handle: &CircleHandle, varying: &CircleVarying) {
  for handle in varying.relationships.iter() {
    update_relationship_change_prediction (accessor, handle);
  }
  update_boundary_change_prediction (accessor, circle_handle);
}

#[derive (Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct BoundaryChange {pub circle_handle: CircleHandle} //, Basics, EventId (0x59732d675b2329ad),
impl PersistentlyIdentifiedType for BoundaryChange {
  const ID: PersistentTypeId = PersistentTypeId(0x6fc5127ff6aeb50d);
}
impl Event for BoundaryChange {
  type Steward = Steward;
  type ExecutionData = ();
  fn execute <Accessor: EventAccessor <Steward = Self::Steward>> (&self, accessor: &mut Accessor) {
    let mut new = tracking_query (accessor, &self.circle_handle.varying);
    
    new.position.update_by(accessor.now() - new.last_change);
    new.last_change = accessor.now().clone();
    if let Some(induced_acceleration) = new.boundary_induced_acceleration {
      new.position.add_acceleration(-induced_acceleration);
      new.boundary_induced_acceleration = None;
    } else {
      let acceleration = -(new.position.evaluate() -
                            Vector2::new(ARENA_SIZE / 2,
                                         ARENA_SIZE / 2)) *
                          (ARENA_SIZE * 400 / (ARENA_SIZE - self.circle_handle.radius));
      new.position.add_acceleration(acceleration);
      new.boundary_induced_acceleration = Some(acceleration);
    }
    set (accessor, &self.circle_handle.varying, new.clone());
    update_predictions (accessor, &self.circle_handle, & new);
    SimpleGridDetector::changed_course(accessor, accessor.globals().detector, & self.circle_handle);
  }

  fn undo <Accessor: FutureCleanupAccessor <Steward = Self::Steward>> (&self, accessor: &mut Accessor, _: ()) {
    unimplemented!()
  }
}

#[derive (Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct Initialize {} //, Basics, EventId (0xa2a17317b84f96e5),
impl PersistentlyIdentifiedType for Initialize {
  const ID: PersistentTypeId = PersistentTypeId(0xbf7ba1ff2ab76640);
}
impl Event for Initialize {
  type Steward = Steward;
  type ExecutionData = ();
  fn execute <Accessor: EventAccessor <Steward = Self::Steward>> (&self, accessor: &mut Accessor) {
    set (accessor, &accessor.globals().detector, accessor::new_handle (SimpleGridDetector::new (Space)));
    let circles = accessor.globals();
    let mut varying = Vec::new();
    let mut generator = DeterministicRandomId::new (&2u8).to_rng();
    let thingy = ARENA_SIZE / 20;
    for index in 0..HOW_MANY_CIRCLES {
      let position = QuadraticTrajectory::new(TIME_SHIFT,
                              MAX_DISTANCE_TRAVELED_AT_ONCE,
                              [generator.gen_range(0, ARENA_SIZE),
                               generator.gen_range(0, ARENA_SIZE),
                               generator.gen_range(-thingy, thingy),
                               generator.gen_range(-thingy, thingy),
                               0,
                               0]);
      varying.push (CircleVarying {
        position: position,
        last_change: 0,
        relationships: Vec::new(),
        boundary_induced_acceleration: None,
        next_boundary_change: None,
      })
      set (accessor, & circles [index].varying, varying [index].clone());
    }
    for index in 0..HOW_MANY_CIRCLES {
      accessor.globals().detector.insert (accessor, Space, & circles [index], None);
    }
  }

  fn undo <Accessor: FutureCleanupAccessor <Steward = Self::Steward>> (&self, accessor: &mut Accessor, _: ()) {
    unimplemented!()
  }
}


#[derive (Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct Disturb {pub coordinates: [SpaceCoordinate; 2]} //, Basics, EventId (0x058cb70d89116605),
impl PersistentlyIdentifiedType for Disturb {
  const ID: PersistentTypeId = PersistentTypeId(0xb8bbf65eaaf08d0e);
}
impl Event for Disturb {
  type Steward = Steward;
  type ExecutionData = ();
  fn execute <Accessor: EventAccessor <Steward = Self::Steward>> (&self, accessor: &mut Accessor) {
    let circles = &accessor.globals().circles;
    let mut best_handle = None;
    {
    let mut best_distance_squared = i64::max_value();
    for circle in circles.iter() {
      let varying = tracking_query_ref (accessor, &circle.varying);
      let position = varying.position.updated_by(accessor.now() - varying.last_change).unwrap().evaluate();
      let distance_squared = (self.coordinates [0] - position [0]) * (self.coordinates [0] - position [0]) + (self.coordinates [1] - position [1]) * (self.coordinates [1] - position [1]);
      if distance_squared <best_distance_squared {
        best_distance_squared = distance_squared;
        best_handle = Some (circle.clone());
      }
    }
    }
    
    let best_handle = best_handle.unwrap() ;
    let mut new;
    {
    let best = tracking_query_ref (accessor, & best_handle.varying);
    new = best.clone();
    new.position.update_by(accessor.now() - best.last_change);
    new.last_change = accessor.now().clone();
    let impulse = -(new.position.evaluate() -
                            Vector2::new(ARENA_SIZE / 2,
                                         ARENA_SIZE / 2)) *
                          (ARENA_SIZE * 4 / (ARENA_SIZE ));
    new.position.add_velocity(impulse);
    }
    set (accessor, & best_handle.varying, new.clone());
    update_predictions (accessor, & best_handle, & new);
    SimpleGridDetector::changed_course(accessor, accessor.globals().detector, & best_handle);
  }

  fn undo <Accessor: FutureCleanupAccessor <Steward = Self::Steward>> (&self, accessor: &mut Accessor, _: ()) {
    unimplemented!()
  }
}

pub fn make_globals()-> <Basics as BasicsTrait>::Globals {
  let mut circles = Vec::new();
  let mut generator = DeterministicRandomId::new (&0u8).to_rng();
  
  for index in 0..HOW_MANY_CIRCLES {
    let radius = generator.gen_range(ARENA_SIZE / 30, ARENA_SIZE / 15);

    circles.push (DataHandle::new_for_globals (Circle {
      index: index,
      radius: radius,
      varying: DataTimelineCell::new(SimpleTimeline::new ())
    }));
  }
  Globals {
    circles: circles,
    detector: DataTimelineCell::new(SimpleTimeline::new ()),
  }
}

