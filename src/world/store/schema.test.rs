use super::{CHECKPOINT_SCHEMA, RECORD_SCHEMA, Schema, SchemaError};
use crate::value::Object;

#[test]
fn a_schema_of_each_minor_of_the_major_opens_and_one_of_another_major_is_refused_with_both() {
  let ours = Schema { major: 2, minor: 2 };
  for held in ["2.0", "2.2", "2.9"].map(Object::string) {
    assert_eq!(ours.opens("record", Some(held.as_ref())), Ok(()), "{}", held.as_ref().py_repr());
  }
  let refused = ours.opens("checkpoint", Some(Object::string("3.0").as_ref())).unwrap_err();
  assert_eq!(
    refused.to_string(),
    "the checkpoint holds the schema 3.0, of another major than the schema 2.2 that this build \
     writes: open it with a build that writes the major 3, or start a new life"
  );
  assert_eq!(
    ours.opens("record", Some(Object::string("later").as_ref())),
    Err(SchemaError::Unread { of: "record", held: "'later'".to_owned() })
  );
}

#[test]
fn each_schema_names_itself_as_its_major_and_its_minor() {
  for one in [RECORD_SCHEMA, CHECKPOINT_SCHEMA] {
    assert_eq!(Schema::read(one.object().as_ref()), Some(one));
  }
}

#[test]
fn a_schema_from_before_the_stable_cut_opens_in_no_life() {
  for held in [None, Some(Object::none()), Some(Object::string("1.1")), Some(Object::string("2.0"))]
  {
    assert_eq!(
      RECORD_SCHEMA.opens("record", held.as_ref().map(Object::as_ref)),
      Err(SchemaError::BeforeCut { of: "record" })
    );
  }
}
