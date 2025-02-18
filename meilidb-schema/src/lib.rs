use std::collections::{BTreeMap, HashMap};
use std::ops::BitOr;
use std::sync::Arc;
use std::{fmt, u16};

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

pub const DISPLAYED: SchemaProps = SchemaProps {
    displayed: true,
    indexed: false,
    ranked: false,
};
pub const INDEXED: SchemaProps = SchemaProps {
    displayed: false,
    indexed: true,
    ranked: false,
};
pub const RANKED: SchemaProps = SchemaProps {
    displayed: false,
    indexed: false,
    ranked: true,
};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaProps {
    #[serde(default)]
    pub displayed: bool,

    #[serde(default)]
    pub indexed: bool,

    #[serde(default)]
    pub ranked: bool,
}

impl SchemaProps {
    pub fn is_displayed(self) -> bool {
        self.displayed
    }

    pub fn is_indexed(self) -> bool {
        self.indexed
    }

    pub fn is_ranked(self) -> bool {
        self.ranked
    }
}

impl BitOr for SchemaProps {
    type Output = Self;

    fn bitor(self, other: Self) -> Self::Output {
        SchemaProps {
            displayed: self.displayed | other.displayed,
            indexed: self.indexed | other.indexed,
            ranked: self.ranked | other.ranked,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct SchemaBuilder {
    identifier: String,
    attributes: IndexMap<String, SchemaProps>,
}

impl SchemaBuilder {
    pub fn with_identifier<S: Into<String>>(name: S) -> SchemaBuilder {
        SchemaBuilder {
            identifier: name.into(),
            attributes: IndexMap::new(),
        }
    }

    pub fn new_attribute<S: Into<String>>(&mut self, name: S, props: SchemaProps) -> SchemaAttr {
        let len = self.attributes.len();
        if self.attributes.insert(name.into(), props).is_some() {
            panic!("Field already inserted.")
        }
        SchemaAttr(len as u16)
    }

    pub fn build(self) -> Schema {
        let mut attrs = HashMap::new();
        let mut props = Vec::new();

        for (i, (name, prop)) in self.attributes.into_iter().enumerate() {
            attrs.insert(name.clone(), SchemaAttr(i as u16));
            props.push((name, prop));
        }

        let identifier = self.identifier;
        Schema {
            inner: Arc::new(InnerSchema {
                identifier,
                attrs,
                props,
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Schema {
    inner: Arc<InnerSchema>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct InnerSchema {
    identifier: String,
    attrs: HashMap<String, SchemaAttr>,
    props: Vec<(String, SchemaProps)>,
}

impl Schema {
    fn to_builder(&self) -> SchemaBuilder {
        let identifier = self.inner.identifier.clone();
        let attributes = self.attributes_ordered();
        SchemaBuilder {
            identifier,
            attributes,
        }
    }

    fn attributes_ordered(&self) -> IndexMap<String, SchemaProps> {
        let mut ordered = BTreeMap::new();
        for (name, attr) in &self.inner.attrs {
            let (_, props) = self.inner.props[attr.0 as usize];
            ordered.insert(attr.0, (name, props));
        }

        let mut attributes = IndexMap::with_capacity(ordered.len());
        for (_, (name, props)) in ordered {
            attributes.insert(name.clone(), props);
        }

        attributes
    }

    pub fn props(&self, attr: SchemaAttr) -> SchemaProps {
        let (_, props) = self.inner.props[attr.0 as usize];
        props
    }

    pub fn identifier_name(&self) -> &str {
        &self.inner.identifier
    }

    pub fn attribute<S: AsRef<str>>(&self, name: S) -> Option<SchemaAttr> {
        self.inner.attrs.get(name.as_ref()).cloned()
    }

    pub fn attribute_name(&self, attr: SchemaAttr) -> &str {
        let (name, _) = &self.inner.props[attr.0 as usize];
        name
    }

    pub fn iter<'a>(&'a self) -> impl Iterator<Item = (&str, SchemaAttr, SchemaProps)> + 'a {
        self.inner.props.iter().map(move |(name, prop)| {
            let attr = self.inner.attrs.get(name).unwrap();
            (name.as_str(), *attr, *prop)
        })
    }
}

impl Serialize for Schema {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        self.to_builder().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Schema {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        let builder = SchemaBuilder::deserialize(deserializer)?;
        Ok(builder.build())
    }
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct SchemaAttr(pub u16);

impl SchemaAttr {
    pub const fn new(value: u16) -> SchemaAttr {
        SchemaAttr(value)
    }

    pub const fn min() -> SchemaAttr {
        SchemaAttr(u16::min_value())
    }

    pub const fn max() -> SchemaAttr {
        SchemaAttr(u16::max_value())
    }

    pub fn next(self) -> Option<SchemaAttr> {
        self.0.checked_add(1).map(SchemaAttr)
    }

    pub fn prev(self) -> Option<SchemaAttr> {
        self.0.checked_sub(1).map(SchemaAttr)
    }
}

impl fmt::Display for SchemaAttr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Diff {
    IdentChange {
        old: String,
        new: String,
    },
    AttrMove {
        name: String,
        old: usize,
        new: usize,
    },
    AttrPropsChange {
        name: String,
        old: SchemaProps,
        new: SchemaProps,
    },
    NewAttr {
        name: String,
        pos: usize,
        props: SchemaProps,
    },
    RemovedAttr {
        name: String,
    },
}

pub fn diff(old: &Schema, new: &Schema) -> Vec<Diff> {
    use Diff::{AttrMove, AttrPropsChange, IdentChange, NewAttr, RemovedAttr};

    let mut differences = Vec::new();
    let old = old.to_builder();
    let new = new.to_builder();

    // check if the old identifier differs from the new one
    if old.identifier != new.identifier {
        let old = old.identifier;
        let new = new.identifier;
        differences.push(IdentChange { old, new });
    }

    // compare all old attributes positions
    // and properties with the new ones
    for (pos, (name, props)) in old.attributes.iter().enumerate() {
        match new.attributes.get_full(name) {
            Some((npos, _, nprops)) => {
                if pos != npos {
                    let name = name.clone();
                    differences.push(AttrMove {
                        name,
                        old: pos,
                        new: npos,
                    });
                }
                if props != nprops {
                    let name = name.clone();
                    differences.push(AttrPropsChange {
                        name,
                        old: *props,
                        new: *nprops,
                    });
                }
            }
            None => differences.push(RemovedAttr { name: name.clone() }),
        }
    }

    // retrieve all attributes that
    // were not present in the old schema
    for (pos, (name, props)) in new.attributes.iter().enumerate() {
        if !old.attributes.contains_key(name) {
            let name = name.clone();
            differences.push(NewAttr {
                name,
                pos,
                props: *props,
            });
        }
    }

    differences
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn difference() {
        use Diff::{AttrMove, AttrPropsChange, IdentChange, NewAttr, RemovedAttr};

        let mut builder = SchemaBuilder::with_identifier("id");
        builder.new_attribute("alpha", DISPLAYED);
        builder.new_attribute("beta", DISPLAYED | INDEXED);
        builder.new_attribute("gamma", INDEXED);
        builder.new_attribute("omega", INDEXED);
        let old = builder.build();

        let mut builder = SchemaBuilder::with_identifier("kiki");
        builder.new_attribute("beta", DISPLAYED | INDEXED);
        builder.new_attribute("alpha", DISPLAYED | INDEXED);
        builder.new_attribute("delta", RANKED);
        builder.new_attribute("gamma", DISPLAYED);
        let new = builder.build();

        let differences = diff(&old, &new);
        let expected = &[
            IdentChange {
                old: format!("id"),
                new: format!("kiki"),
            },
            AttrMove {
                name: format!("alpha"),
                old: 0,
                new: 1,
            },
            AttrPropsChange {
                name: format!("alpha"),
                old: DISPLAYED,
                new: DISPLAYED | INDEXED,
            },
            AttrMove {
                name: format!("beta"),
                old: 1,
                new: 0,
            },
            AttrMove {
                name: format!("gamma"),
                old: 2,
                new: 3,
            },
            AttrPropsChange {
                name: format!("gamma"),
                old: INDEXED,
                new: DISPLAYED,
            },
            RemovedAttr {
                name: format!("omega"),
            },
            NewAttr {
                name: format!("delta"),
                pos: 2,
                props: RANKED,
            },
        ];

        assert_eq!(&differences, expected)
    }

    #[test]
    fn serialize_deserialize() -> bincode::Result<()> {
        let mut builder = SchemaBuilder::with_identifier("id");
        builder.new_attribute("alpha", DISPLAYED);
        builder.new_attribute("beta", DISPLAYED | INDEXED);
        builder.new_attribute("gamma", INDEXED);
        let schema = builder.build();

        let mut buffer = Vec::new();
        bincode::serialize_into(&mut buffer, &schema)?;
        let schema2 = bincode::deserialize_from(buffer.as_slice())?;

        assert_eq!(schema, schema2);

        Ok(())
    }

    #[test]
    fn serialize_deserialize_toml() -> Result<(), Box<dyn Error>> {
        let mut builder = SchemaBuilder::with_identifier("id");
        builder.new_attribute("alpha", DISPLAYED);
        builder.new_attribute("beta", DISPLAYED | INDEXED);
        builder.new_attribute("gamma", INDEXED);
        let schema = builder.build();

        let buffer = toml::to_vec(&schema)?;
        let schema2 = toml::from_slice(buffer.as_slice())?;

        assert_eq!(schema, schema2);

        let data = r#"
            identifier = "id"

            [attributes."alpha"]
            displayed = true

            [attributes."beta"]
            displayed = true
            indexed = true

            [attributes."gamma"]
            indexed = true
        "#;
        let schema2 = toml::from_str(data)?;
        assert_eq!(schema, schema2);

        Ok(())
    }

    #[test]
    fn serialize_deserialize_json() -> Result<(), Box<dyn Error>> {
        let mut builder = SchemaBuilder::with_identifier("id");
        builder.new_attribute("alpha", DISPLAYED);
        builder.new_attribute("beta", DISPLAYED | INDEXED);
        builder.new_attribute("gamma", INDEXED);
        let schema = builder.build();

        let buffer = serde_json::to_vec(&schema)?;
        let schema2 = serde_json::from_slice(buffer.as_slice())?;

        assert_eq!(schema, schema2);

        let data = r#"
            {
                "identifier": "id",
                "attributes": {
                    "alpha": {
                        "displayed": true
                    },
                    "beta": {
                        "displayed": true,
                        "indexed": true
                    },
                    "gamma": {
                        "indexed": true
                    }
                }
            }"#;
        let schema2 = serde_json::from_str(data)?;
        assert_eq!(schema, schema2);

        Ok(())
    }
}
