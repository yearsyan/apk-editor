use std::error::Error;
use std::io::Write;
use crate::manifest::axml::{AndroidXml, StringChunkBuilder, XmlAttributeValue, XmlNode};

pub struct AndroidManifest<'a> {
    xml: AndroidXml<'a>,
    string_chunk_builder: StringChunkBuilder,
    application_node_index: usize
}

pub struct Activity {
    pub class_name: String,
}

pub struct Provider {
    pub class_name: String,
    pub authorities: String
}

impl<'a> AndroidManifest<'a> {
    pub fn from(data: &'a Vec<u8>) -> Result<Self, Box<dyn Error>> {
        let mut res = AndroidManifest{
            xml: AndroidXml::from_data(data)?,
            string_chunk_builder: StringChunkBuilder::new(),
            application_node_index: 0
        };
        for (index, node) in res.xml.content.root_node.children.iter().enumerate() {
            if node.tag_name == "application" {
                res.application_node_index = index;
                break;
            }
        }
        res.string_chunk_builder.init(&mut res.xml.string_chunk);
        Ok(res)
    }

    pub fn write<W: Write>(&self, writer: W) -> Result<(), std::io::Error> {
        // TODO
        Ok(())
    }

    pub fn get_data(&mut self) -> Vec<u8> {
        self.xml.regenerate(&mut self.string_chunk_builder)
    }

    pub fn add_content_provider(&mut self, cp: Provider) {
        let application = self.xml.content.root_node.children[self.application_node_index].as_mut();
        let name_value_index = self.string_chunk_builder.put(cp.class_name.as_str());
        let authorities_value_index = self.string_chunk_builder.put(cp.authorities.as_str());
        application.children.push(Box::new(XmlNode{
            tag_name: String::from("provider"),
            attrs: vec![XmlAttributeValue{
                namespace_uri: Some("http://schemas.android.com/apk/res/android".to_string()),
                name_index: 3,
                name: "name".to_string(),
                value_type: 0x3000008,
                string_data: Some(cp.class_name),
                data: name_value_index
            }, XmlAttributeValue{
                namespace_uri: Some("http://schemas.android.com/apk/res/android".to_string()),
                name_index: 5,
                name: "authorities".to_string(),
                value_type: 0x3000008,
                string_data: Some(cp.authorities),
                data: authorities_value_index
            }],
            children: vec![]
        }));
    }

    pub fn add_activity(&mut self, activity: Activity) {
        let application = self.xml.content.root_node.children[self.application_node_index].as_mut();
        let value_index = self.string_chunk_builder.put(activity.class_name.as_str());
        application.children.push(Box::new(XmlNode{
            tag_name: String::from("activity"),
            attrs: vec![XmlAttributeValue{
                namespace_uri: Some("http://schemas.android.com/apk/res/android".to_string()),
                name_index: 3,
                name: "name".to_string(),
                value_type: 0x3000008,
                string_data: Some(activity.class_name),
                data: value_index
            }],
            children: vec![]
        }));
    }

}


