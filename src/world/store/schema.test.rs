use super::{CHECKPOINT_SCHEMA, RECORD_SCHEMA, Schema, SchemaError};
use crate::value::Object;

#[test]
fn a_schema_of_each_minor_of_the_major_opens_and_one_of_another_major_is_refused_with_both() {
  let ours = Schema { major: 1, minor: 2 };
  for held in ["1.0", "1.2", "1.9"].map(Object::string).into_iter().chain([Object::int(1)]) {
    assert_eq!(ours.opens("record", Some(held.as_ref())), Ok(()), "{}", held.as_ref().py_repr());
  }
  assert_eq!(ours.opens("record", None), Ok(()), "a record before the schemas opens");
  let refused = ours.opens("checkpoint", Some(Object::string("2.0").as_ref())).unwrap_err();
  assert_eq!(
    refused.to_string(),
    "the checkpoint holds the schema 2.0, of another major than the schema 1.2 that this build \
     writes: open it with a build that writes the major 2, or start a new life"
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
