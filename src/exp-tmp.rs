use yaserde::{YaDeserialize, YaSerialize};

fn main() {
    let foo: Foo = yaserde::de::from_str(XML).unwrap();
    println!("foo: {foo:?}");
    let cfg = yaserde::ser::Config{
        perform_indent: true,
        write_document_declaration: true,
        indent_string: Some("  ".to_string()),
    };
    let s = yaserde::ser::to_string_with_config(&foo, &cfg).unwrap();
    println!("s: {s}");
}

#[derive(YaDeserialize, YaSerialize, Debug)]
// #[derive(YaDeserialize, Debug)]
struct Foo {
    #[yaserde(child)]
    pub bar: i64,
    #[yaserde(child)]
    pub baz: raw::OpaqueXml,
    #[yaserde(child)]
    pub bah: String,
}

const XML: &str = r#"
<Foo>
 <bar>123</bar>
 <baz x="7"><bleh>flab</bleh><zig zag="8"/><fee><foo/><fum/></fee></baz>
 <bah>hello!</bah>
</Foo>
"#;

mod raw {
    use xml::reader::XmlEvent;
    use yaserde::{YaDeserialize, YaSerialize};

    #[derive(Debug)]
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
                    EndElement{..} if depth == start_depth+1 => {
                        events.push(peek.clone());
                        break;
                    },
                    _ => {},
                }
                events.push(reader.next_event()?);
            }
            Ok(Self{events})
        }
    }

    impl YaSerialize for OpaqueXml {
        fn serialize<W>(&self, writer: &mut yaserde::ser::Serializer<W>) -> Result<(), String>
        where
            W: std::io::Write,
        {
            for ev in &self.events {
                writer.write(ev.as_writer_event().unwrap()).unwrap();
                // if let Some(ev) = ev.as_writer_event() {
                // }
            }
            Ok(())
        }

        fn serialize_attributes(
            &self,
            source_attributes: Vec<xml::attribute::OwnedAttribute>,
            source_namespace: xml::namespace::Namespace,
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
}
