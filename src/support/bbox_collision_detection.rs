use ::{DeterministicRandomId, PersistentTypeId, ListedType, PersistentlyIdentifiedType, SimulationStateData, DataHandleTrait, DataTimelineCellTrait, ExtendedTime};
use ::stewards::{simple_full as steward_module};
use self::steward_module::{TimeSteward, Event, DataTimelineCellReadGuard, DataHandle, DataTimelineCell, Accessor, EventAccessor, FutureCleanupAccessor, SnapshotAccessor, simple_timeline};
use self::simple_timeline::{SimpleTimeline, GetVarying, tracking_query, tracking_query_ref, set, unset};


pub type Coordinate = u32;
pub type NumDimensions = u32;

/// If there's only one interpretation of how your objects are arranged in space, it makes sense to implement this on a unit-like struct.
pub trait Space: SimulationStateData + PersistentlyIdentifiedType {
  type Steward: TimeSteward;
  type Object: SimulationStateData + PersistentlyIdentifiedType;
  type DetectorDataPerObject: SimulationStateData;
  
  const DIMENSIONS: NumDimensions;

  // An Object generally has to store some opaque data for the collision detector.
  // It would normally include a DataHandle to a tree node.
  // These are getter and setter methods for that data.
  fn get_detector_data<A: EventAccessor <Steward = Self::Steward>>(&self, accessor: &A, object: &DataHandle<Self::Object>)->Option<&DetectorDataPerObject<Self>>>;
  fn set_detector_data<A: EventAccessor <Steward = Self::Steward>>(&self, accessor: &A, object: &DataHandle<Self::Object>, data: Option<DetectorDataPerObject<Self>>);

  fn current_bounding_box<A: EventAccessor <Steward = Self::Steward>>(&self, accessor: &A, object: &DataHandle<Self::Object>)->BoundingBox;
  fn when_escapes<A: EventAccessor <Steward = Self::Steward>>(&self, accessor: &A, object: &DataHandle<Self::Object>, space: &Self::Space, BoundingBox)-><Self::Steward as TimeSteward>::Basics::Time;
  
  fn become_neighbors<A: EventAccessor <Steward = Self::Steward>>(&self, accessor: &A, objects: [&DataHandle<Self::Object>; 2]) {}
  fn stop_being_neighbors<A: EventAccessor <Steward = Self::Steward>>(&self, accessor: &A, objects: [&DataHandle<Self::Object>; 2]) {}
}

pub trait Detector: SimulationStateData {
  type Steward: TimeSteward;
  type Object: SimulationStateData + PersistentlyIdentifiedType;
  type DetectorDataPerObject: SimulationStateData;

  pub fn insert<A: EventAccessor <Steward = Self::Steward>, S: Space <DetectorDataPerObject = Self::DetectorDataPerObject>>(accessor: &A, space: &S, object: &DataHandle<S::Object>, location_hint: Option <& DataHandle<B::Object>>);
  pub fn remove<A: EventAccessor <Steward = Self::Steward>, S: Space <DetectorDataPerObject = Self::DetectorDataPerObject>>(accessor: &A, space: &S, object: &DataHandle<S::Object>);
  pub fn changed<A: EventAccessor <Steward = Self::Steward>, S: Space <DetectorDataPerObject = Self::DetectorDataPerObject>>(accessor: &A, space: &S, object: &DataHandle<S::Object>);
  pub fn nearby_objects<A: EventAccessor <Steward = Self::Steward>, S: Space <DetectorDataPerObject = Self::DetectorDataPerObject>>(accessor: &A, space: &S, object: &DataHandle<S::Object>) ->Vec<DataHandle<S::Object>>;
}

pub struct BoundingBox<B: BoundingBoxCollisionDetectable> {
  pub bounds: [[Coordinate; 2]; B::DIMENSIONS],
}





pub mod simple_grid {
  use super::*;
  
  pub struct SimpleGridDetector <Steward: TimeSteward> {
    cell_size: Coordinate,
    cells: DataTimelineCell<SimpleTimeline<HashMap <[Coordinate; DIMENSIONS], Cell>>>,
  }
  
  struct DetectorDataPerObject {
    current_grid_bounds: BoundingBox,
    escapes_bounds_prediction: Option <<Steward as TimeSteward>::EventHandle>,
  }
  struct Cell {
    objects: Vec<Object>,
  }
  
  impl Detector for SimpleGridDetector {
    type Steward = Steward;
    type Object = Object;
    type DetectorDataPerObject = DetectorDataPerObject;
    
    insert (accessor, space, object, location_hint) {
      
    }
    
    changed (accessor, space, object) {
      let data = match space.get_detector_data (accessor, object) {None => return, Some (a) => a};
      let new_bounds = space.current_bounding_box (accessor, object);
      let new_grid_bounds = self.grid_box (new_bounds);
      
      let mut cells = get (accessor, self.cells);
      let mut new_neighbors =
      let mut removed_neighbors =
      
      for location in new_bounds.locations () {
        if let Some(cell) = cells.get_mut (location) {
          for neighbor in cell.objects {
            new_neighbors.insert (neighbor) ;
          }
          if !old_bounds.contains (location) {
            cell.objects.insert (object.clone());
          }
        }
      }
      for location in old_bounds.locations () {
        if !new_bounds.contains (location) {
          let cell = cells.entry (location).or_insert (Default::default());
          for neighbor in cell.objects {
            if !new_neighbors.contains (neighbor) {
              removed_neighbors.insert (neighbor);
            }
          }
          cell.objects.insert (object.clone());
        }
      }
      
      for neighbor in removed_neighbors {
        space.stop_being_neighbors (accessor, [object, neighbor]);
      }
      for neighbor in new_neighbors {
        space.become_neighbors (accessor, [object, neighbor]);
      }
      
      
    }
  }
  
  impl SimpleGridDetector {
    fn grid_box (&self, exact_box: & BoundingBox) -> BoundingBox {
      Array::from_fn (| dimension | Array::from_fn (| direction | {
        exact_box [dimension] [direction] + direction*(self.cell_size - 1)/self.cell_size
      }))
    }
    fn real_box_from_grid (&self, grid_box: & BoundingBox) -> BoundingBox {
      Array::from_fn (| dimension | Array::from_fn (| direction | {
        grid_box [dimension] [direction] * self.cell_size
      }))
    }
  }
}






/*
struct NodeBounds<B: BoundingBoxCollisionDetectable> {
  half_size_shift: u32,
  center: [Coordinate; B::DIMENSIONS],
}
fn smallest_containing_node_bounds <B: BoundingBoxCollisionDetectable> (bounds: & BoundingBox<B>)->NodeBounds <B> {
  unimplemented!()
}

#[derive (Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
struct Node<B: BoundingBoxCollisionDetectable> {
  bounds: NodeBounds<B>,
  parent: Option<DataHandle <Node <B>>>,
  larger_cousins: [[Option<DataHandle <Node <B>>>; 2]; B::DIMENSIONS],
  varying: NodeVarying <B>,
}
impl<B: BoundingBoxCollisionDetectable> PersistentlyIdentifiedType for Node<B> {
  const ID: PersistentTypeId = PersistentTypeId(B::ID.0 ^ 0x7c4c8993671ba023);
}
#[derive (Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
struct NodeVarying<B: BoundingBoxCollisionDetectable> {
  children: [Option<DataHandle <Node <B>>>; 1<<B::DIMENSIONS],
  overlapping_live_nodes: Vec<DataHandle <B::Object>>,
  objects: Vec<DataHandle <B::Object>>,
}

pub struct ObjectData<B: BoundingBoxCollisionDetectable> {
  varying: ObjectVarying<B>,
}
struct ObjectVarying<B: BoundingBoxCollisionDetectable> {
  node: DataHandle <Node <B>>,
}
pub struct BoundingBoxCollisionDetector<B: BoundingBoxCollisionDetectable> {
  root: DataHandle <Node <B>>,
}

impl<B: BoundingBoxCollisionDetectable> BoundingBoxCollisionDetector<B> {
  pub fn insert<A: EventAccessor <Steward = B::Steward>>(accessor: &A, object: &DataHandle<B::Object>, space: &B::Space, location_hint: Option <& DataHandle<B::Object>>) {
  
  }
  pub fn remove<A: EventAccessor <Steward = B::Steward>>(accessor: &A, object: &DataHandle<B::Object>, space: &B::Space) {
  
  }
  pub fn neighbors<A: EventAccessor <Steward = B::Steward>>(accessor: &A, object: &DataHandle<B::Object>, space: &B::Space) -> Iter {
  
  }
}


#[derive (Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct EscapesBounds<B: BoundingBoxCollisionDetectable> {}
impl<B: BoundingBoxCollisionDetectable> PersistentlyIdentifiedType for EscapesBounds<B> {
  const ID: PersistentTypeId = PersistentTypeId(B::ID.0 ^ 0xb0cdbe951b688b70);
}
impl<B: BoundingBoxCollisionDetectable> Event for EscapesBounds<B> {
  type Steward = B::Steward;
  type ExecutionData = ();
  fn execute <Accessor: EventAccessor <Steward = Self::Steward>> (&self, accessor: &mut Accessor) {

  }

  fn undo <Accessor: FutureCleanupAccessor <Steward = Self::Steward>> (&self, accessor: &mut Accessor, _: ()) {
    unimplemented!()
  }
}


impl<B: BoundingBoxCollisionDetectable> BoundingBoxCollisionDetector<B> {
  fn reposition <A: EventAccessor <Steward = B::Steward>> (accessor: &A, object: &DataHandle<B::Object>) {
    let varying = tracking_query (accessor, object.varying);
    let mut current_node = varying.node;
    let new_bounds = B::calculate_current_bounding_box (object);
    let destination_node_bounds = smallest_containing_node_bounds (new_bounds) ;
    let bigger_shift = max (current_node.bounds.half_size_shift, destination_node_bounds.half_size_shift);
    
    while current_node.bounds.half_size_shift < bigger_shift {
      current_node = current_node.parent.expect("All nodes except the root node have to have a parent, and we should never be trying to navigate to a node bigger than the root");
    }
    
    let mut destination_ancestor_bounds = destination_node_bounds;
    while destination_ancestor_bounds.half_size_shift < bigger_shift {
      destination_ancestor_bounds = parent_bounds(destination_ancestor_bounds);
    }
    
    while largest_single_dimension_distance (current_node.bounds, destination_ancestor_bounds) > (1 << destination_ancestor_bounds.half_size_shift) {
      destination_ancestor_bounds = parent_bounds(destination_ancestor_bounds);
      current_node = current_node.parent.expect("All nodes except the root node have to have a parent, and we should never be trying to navigate to a node bigger than the root");
    }
    
    if current_node.bounds != destination_ancestor_bounds {
      // they should now be the same size and half- or all-overlapping in each dimension.
    }
  }
}

*/
