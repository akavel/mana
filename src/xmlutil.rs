use xml::reader::XmlEvent;
use yaserde::{YaDeserialize, YaSerialize};

#[derive(Clone, Debug)]
pub struct OpaqueXml {
    events: Vec<XmlEvent>,
}

impl YaDeserialize for OpaqueXml {
    fn deserialize<R>(reader: &mut yaserde::de::Deserializer<R>) -> Result<Self, String>
    where
        R: std::io::Read,
    {
        use xml::reader::XmlEvent::*;
        let mut events = vec![];
        let start_depth = reader.depth();
        loop {
            let depth = reader.depth();
            let peek = reader.peek()?;
            match peek {
                EndElement { .. } if depth == start_depth + 1 => {
                    events.push(peek.clone());
                    break;
                }
                _ => {}
            }
            events.push(reader.next_event()?);
        }
        Ok(Self { events })
    }
}

impl YaSerialize for OpaqueXml {
    fn serialize<W>(&self, writer: &mut yaserde::ser::Serializer<W>) -> Result<(), String>
    where
        W: std::io::Write,
    {
        for ev in &self.events {
            writer.write(ev.as_writer_event().unwrap()).unwrap();
        }
        Ok(())
    }

    fn serialize_attributes(
        &self,
        _source_attributes: Vec<xml::attribute::OwnedAttribute>,
        _source_namespace: xml::namespace::Namespace,
    ) -> Result<
        (
            Vec<xml::attribute::OwnedAttribute>,
            xml::namespace::Namespace,
        ),
        String,
    > {
        Err("OpaqueXml cannot be serialized as attribute".to_string())
        // Ok((source_attributes, source_namespace))
    }
}
