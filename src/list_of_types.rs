use std::collections::HashMap;
use serde::{Serialize, Serializer, Error, de};

use {ExtendedTime, StewardRc, FieldRc, ColumnId, Column, Basics};

macro_rules! type_list_definitions {
($module: ident, $Trait: ident, $IdType: ident, $get_id: ident) => {
pub mod $module {
use std::any::Any;
use std::marker::PhantomData;
use {$Trait,$IdType};

pub type Id = $IdType;
pub use $Trait as Trait;
pub fn get_id <T: $Trait>()->Id {T::$get_id()}

enum Void {}
pub struct Item <T: $Trait>(PhantomData <T>, Void);
pub trait User {
  fn apply<T: $Trait>(&mut self);
}
pub trait List: Any {
  fn apply<U: User>(user: &mut U);
}
impl<T: Any> List for T {
  #[inline]
  default fn apply<U: User>(_: &mut U) {}
}
impl<T: $Trait> List for Item <T> {
  #[inline]
  fn apply<U: User>(user: &mut U) {
    user.apply::<T>();
  }
}

tuple_impls! (T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24, T25, T26, T27, T28, T29, T30, T31);
}
};

($module: ident, $Trait: ident <B>, $IdType: ident, $get_id: ident) => {
pub mod $module {
use std::any::Any;
use std::marker::PhantomData;
use {$Trait,$IdType, Basics};

pub type Id = $IdType;
pub use $Trait as Trait;
pub fn get_id <T: $Trait>()->Id {T::$get_id()}

enum Void {}
pub struct Item <T: $Trait>(PhantomData <T>, Void);
pub trait User <B: Basics> {
  fn apply<T: $Trait <Basics = B>>(&mut self);
}
pub trait List <B: Basics>: Any {
  fn apply<U: User <B>>(user: &mut U);
}
impl<B: Basics, T: Any> List <B> for T {
  #[inline]
  default fn apply<U: User <B>>(_: &mut U) {}
}
impl<B: Basics, T: $Trait<Basics = B>> List <B> for Item <T> {
  #[inline]
  fn apply<U: User <B>>(user: &mut U) {
    user.apply::<T>();
  }
}
tuple_impls! (B: T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24, T25, T26, T27, T28, T29, T30, T31);
}
};
}
macro_rules! tuple_impls {
  ($TL: ident $(, $T: ident)*) => {
    impl<$($T,)* $TL> List for ($($T,)* $TL,)
      where $($T: List,)* $TL: List
    {
      #[inline]
      fn apply <U: User> (user: &mut U) {
        $($T::apply(user);)*
        $TL::apply(user);
      }
    }
    tuple_impls! ($($T),*);
  };
  () => {};
  (B: $TL: ident $(, $T: ident)*) => {
    impl<B: Basics, $($T,)* $TL> List <B> for ($($T,)* $TL,)
      where $($T: List <B>,)* $TL: List <B>
    {
      #[inline]
      fn apply <U: User <B>> (user: &mut U) {
        $($T::apply(user);)*
        $TL::apply(user);
      }
    }
    tuple_impls! (B: $($T),*);
  };
  (B:) => {};
}
macro_rules! pair_null_impls {
($module0: ident $module1: ident) => {
impl<T: $module0::Trait> $module1::List for $module0::Item <T> {
  #[inline]
  fn apply<U: $module1::User>(_: &mut U) {}
}
impl<T: $module1::Trait> $module0::List for $module1::Item <T> {
  #[inline]
  fn apply<U: $module0::User>(_: &mut U) {}
}
};
}
macro_rules! all_null_impls {
($info0:tt $($info:tt)*) => {
  $(pair_null_impls! ($info0 $info);)*
  all_null_impls! ($($info)*);
};
() => {};
}
macro_rules! all_list_definitions {
($([$($info:tt)*])*) => {
  $(type_list_definitions! ($($info)*);)*
  //all_null_impls! ($([$($info)*])*);
};
}

// Today I Learned that macro hygiene is not applied to type parameter lists
//
// macro_rules! escalate {
// ([$first:tt $($whatever:tt)*] $($T: ident)*) => {escalate! ([$($whatever)*] foo $($T)*);};
// ([] $($T: ident)*) => {tuple_impls! ($($T),*);};
// }
// escalate! ([!!!!!!!! !!!!!!!! !!!!!!!! !!!!!!!!]);
//

all_list_definitions! (
  [column_list, Column, ColumnId, column_id]
  [event_list, Event <B>, EventId, event_id]
  [predictor_list, Predictor <B>, PredictorId, predictor_id]
);
//all_null_impls! (column_list event_list predictor_list);

pub use column_list::List as ColumnList;
pub use column_list::Item as ColumnType;
pub use event_list::List as EventList;
pub use event_list::Item as EventType;
pub use predictor_list::List as PredictorList;
pub use predictor_list::Item as PredictorType;


#[macro_export]
macro_rules! time_steward_make_function_table_type {
  ($module: ident, struct $Struct: ident, fn $memoized_function: ident, fn $function: ident <$T: ident: $Trait: ident $(, [$Parameter: ident $($bounds:tt)*])*> ($($argument_name: ident: $argument_type:ty),*)->$return_type:ty) => {
pub struct $Struct <$($Parameter $($bounds)*),*> (HashMap<$module::Id, fn($($argument_name: $argument_type),*)-> $return_type>);
impl<$($Parameter $($bounds)*),*> $module::User for $Struct<$($Parameter),*> {
  fn apply<$T: $Trait>(&mut self) {
    self.0.insert($module::get_id:: <$T>(), $function::<$T $(, $Parameter)*>);
  }
}
impl<$($Parameter $($bounds)*),*> $Struct<$($Parameter),*> {
  pub fn new <L: $module::List>()->$Struct<$($Parameter),*> {
    let mut result = $Struct (HashMap::new());
    L::apply (&mut result);
    result
  }
  pub fn get (&self, id: $module::Id)->fn ($($argument_type),*)->$return_type {
    *(self.0.get (&id).expect ("Type missing from function table; did you forget to list it in Basics::IncludedTypes?"))
  }
  pub fn call (&self, id: $module::Id $(, $argument_name: $argument_type)*)->$return_type {
    self.get (id)($($argument_name),*)
  }
}

#[allow (unused_imports)]
pub fn $memoized_function <L: $module::List $(, $Parameter $($bounds)*)*> (id: $module::Id $(, $argument_name: $argument_type)*)-> $return_type where L: ::std::any::Any $(, $Parameter: ::std::any::Any)* {
  use std::any::{Any, TypeId};
  use std::cell::RefCell;
  thread_local! {static TABLE: RefCell<HashMap <($(time_steward_make_function_table_type ! (replace_with_typeid $Parameter)),*), Box <Any>>> = RefCell::new (HashMap::new());}
  let function = TABLE.with (| table | {
    table.borrow_mut().entry (($(TypeId::of::<$Parameter>()),*)).or_insert (Box::new ($Struct ::<$($Parameter),*>::new::<L>())).downcast_ref::<$Struct <$($Parameter),*>>().unwrap().get (id)
  });
  function ($($argument_name),*)
}


};
(replace_with_typeid $Parameter: ident) => {TypeId};
}


fn check_equality<C: Column>(first: &FieldRc, second: &FieldRc)->bool {
  ::unwrap_field::<C>(first) == ::unwrap_field::<C>(second)
}
time_steward_make_function_table_type! (column_list, struct FieldEqualityTable, fn fields_are_equal, fn check_equality<C: Column>(first: &FieldRc, second: &FieldRc)->bool);

pub fn field_options_are_equal <C: ColumnList> (column_id: ColumnId, first: Option <& FieldRc>, second: Option <& FieldRc>)->bool {
    match (first, second) {
      (None, None) => true,
      (Some (first), Some (second)) => fields_are_equal::<C> (column_id, first, second),
      _ => false,
    }
}

fn serialize_field <C: Column, S: Serializer>(field: &FieldRc,
                                              serializer: &mut S)
                                              -> Result<(), S::Error> {
  try!(::unwrap_field::<C>(field).serialize(serializer));
  Ok(())
}
fn deserialize_field_from_map <C: Column, B: Basics, M: de::MapVisitor>
  (visitor: &mut M)
   -> Result<(FieldRc, ExtendedTime<B>), M::Error> {
  let (data, time) = try!(visitor.visit_value::<(C::FieldType, ExtendedTime<B>)>());
  Ok((StewardRc::new(data), time))
}
time_steward_make_function_table_type! (column_list, struct FieldSerializationTable, fn serialize_field_rename_this, fn serialize_field <C: Column, [S: Serializer]>(field: &FieldRc,
                                                serializer: &mut S)
                                                -> Result<(), S::Error> );
time_steward_make_function_table_type! (column_list, struct MappedFieldDeserializationTable, fn deserialize_field_rename_this, fn deserialize_field_from_map <C: Column, [B: Basics], [M: de::MapVisitor]>(visitor: &mut M)
                                                -> Result<(FieldRc, ExtendedTime<B>), M::Error>  );

impl<S: Serializer> FieldSerializationTable<S> {
  pub fn serialize_field(&self, column_id: ColumnId, first: & FieldRc,
                                                  serializer: &mut S)->Result<(), S::Error>{
    use serde::ser::Error;
    try!(self.0.get (&column_id).ok_or (S::Error::custom ("Column missing from serialization table; did you forget to list a column in Basics::Columns?"))) (first, serializer)
  }
}
impl<B: Basics, M: de::MapVisitor> MappedFieldDeserializationTable<B, M> {
  pub fn deserialize_field(&self, column_id: ColumnId, visitor: &mut M)->Result<(FieldRc, ExtendedTime<B>), M::Error>{
    try!(self.0.get (&column_id).ok_or (M::Error::custom ("Column missing from deserialization table; did you forget to list a column in Basics::Columns?"))) (visitor)
  }
}


