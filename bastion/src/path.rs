use crate::context::BastionId;
use std::fmt;
use std::result::Result;

#[derive(Clone)]
pub(crate) struct BastionPath {
    // TODO: possibly more effective collection depending on how we'll use it in routing
    parent_chain: Vec<BastionId>,
    this: Option<BastionPathElement>,
}

impl BastionPath {
    // SYSTEM or a sender out of Bastion scope
    pub(crate) fn root() -> BastionPath {
        BastionPath {
            parent_chain: vec![],
            this: None,
        }
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &BastionId> {
        let parent_iter = self.parent_chain.iter();
        parent_iter.chain(self.this.iter().map(|e| e.id()))
    }
}

impl fmt::Display for BastionPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "/{}",
            self.iter()
                .map(|id| format!("{}", id))
                .collect::<Vec<String>>()
                .join("/")
        )
    }
}

impl fmt::Debug for BastionPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.this {
            Some(this @ BastionPathElement::Supervisor(_)) => write!(
                f,
                "/{}",
                self.parent_chain
                    .iter()
                    .map(|id| BastionPathElement::Supervisor(id.clone()))
                    .chain(vec![this.clone()])
                    .map(|el| format!("{:?}", el))
                    .collect::<Vec<String>>()
                    .join("/")
            ),
            // TODO: combine with the pattern above when or-patterns become stable
            Some(this @ BastionPathElement::Children(_)) => write!(
                f,
                "/{}",
                self.parent_chain
                    .iter()
                    .map(|id| BastionPathElement::Supervisor(id.clone()))
                    .chain(vec![this.clone()])
                    .map(|el| format!("{:?}", el))
                    .collect::<Vec<String>>()
                    .join("/")
            ),
            Some(this @ BastionPathElement::Child(_)) => {
                let parent_len = self.parent_chain.len();

                write!(
                    f,
                    "/{}",
                    self.parent_chain
                        .iter()
                        .enumerate()
                        .map(|(i, id)| {
                            if i == parent_len - 1 {
                                BastionPathElement::Children(id.clone())
                            } else {
                                BastionPathElement::Supervisor(id.clone())
                            }
                        })
                        .chain(vec![this.clone()])
                        .map(|el| format!("{:?}", el))
                        .collect::<Vec<String>>()
                        .join("/")
                )
            }
            None => write!(f, "/"),
        }
    }
}

#[derive(Clone)]
pub(crate) enum BastionPathElement {
    Supervisor(BastionId),
    Children(BastionId),
    Child(BastionId),
}

impl fmt::Debug for BastionPathElement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BastionPathElement::Supervisor(id) => write!(f, "supervisor#{}", id),
            BastionPathElement::Children(id) => write!(f, "children#{}", id),
            BastionPathElement::Child(id) => write!(f, "child#{}", id),
        }
    }
}

impl BastionPathElement {
    pub(crate) fn id(&self) -> &BastionId {
        match self {
            BastionPathElement::Supervisor(id) => id,
            BastionPathElement::Children(id) => id,
            BastionPathElement::Child(id) => id,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct AppendError {
    path: BastionPath,
    element: BastionPathElement,
}

impl BastionPath {
    pub(crate) fn append(self, el: BastionPathElement) -> Result<BastionPath, AppendError> {
        match el {
            sv @ BastionPathElement::Supervisor(_) => match self.this {
                None => Ok(BastionPath {
                    parent_chain: self.parent_chain,
                    this: Some(sv),
                }),
                Some(BastionPathElement::Supervisor(id)) => {
                    let mut path = BastionPath {
                        parent_chain: self.parent_chain,
                        this: Some(sv),
                    };
                    path.parent_chain.push(id);
                    Ok(path)
                }
                this => Err(AppendError {
                    path: BastionPath {
                        parent_chain: self.parent_chain,
                        this,
                    },
                    element: sv,
                }),
            },
            children @ BastionPathElement::Children(_) => match self.this {
                Some(BastionPathElement::Supervisor(id)) => {
                    let mut path = BastionPath {
                        parent_chain: self.parent_chain,
                        this: Some(children),
                    };
                    path.parent_chain.push(id);
                    Ok(path)
                }
                this => Err(AppendError {
                    path: BastionPath {
                        parent_chain: self.parent_chain,
                        this,
                    },
                    element: children,
                }),
            },
            child @ BastionPathElement::Child(_) => match self.this {
                Some(BastionPathElement::Children(id)) => {
                    let mut path = BastionPath {
                        parent_chain: self.parent_chain,
                        this: Some(child),
                    };
                    path.parent_chain.push(id);
                    Ok(path)
                }
                this => Err(AppendError {
                    path: BastionPath {
                        parent_chain: self.parent_chain,
                        this,
                    },
                    element: child,
                }),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // SYSTEM + smth

    #[test]
    fn append_sv_to_system() {
        let sv_id = BastionId::new();
        let path = BastionPath::root()
            .append(BastionPathElement::Supervisor(sv_id.clone()))
            .unwrap();
        assert_eq!(path.iter().collect::<Vec<&BastionId>>(), vec![&sv_id]);
    }

    #[test]
    fn append_children_to_system() {
        let sv_id = BastionId::new();
        let res = BastionPath::root().append(BastionPathElement::Children(sv_id));
        assert!(res.is_err())
    }

    #[test]
    fn append_child_to_system() {
        let sv_id = BastionId::new();
        let res = BastionPath::root().append(BastionPathElement::Child(sv_id));
        assert!(res.is_err())
    }

    // Supervisor + smth

    #[test]
    fn append_sv_to_sv() {
        let sv1_id = BastionId::new();
        let sv2_id = BastionId::new();
        let path = BastionPath::root()
            .append(BastionPathElement::Supervisor(sv1_id.clone()))
            .unwrap()
            .append(BastionPathElement::Supervisor(sv2_id.clone()))
            .unwrap();
        assert_eq!(
            path.iter().collect::<Vec<&BastionId>>(),
            vec![&sv1_id, &sv2_id]
        );
    }

    #[test]
    fn append_children_to_sv() {
        let sv_id = BastionId::new();
        let children_id = BastionId::new();
        let path = BastionPath::root()
            .append(BastionPathElement::Supervisor(sv_id.clone()))
            .unwrap()
            .append(BastionPathElement::Children(children_id.clone()))
            .unwrap();
        assert_eq!(
            path.iter().collect::<Vec<&BastionId>>(),
            vec![&sv_id, &children_id]
        );
    }

    #[test]
    fn append_child_to_sv() {
        let sv_id = BastionId::new();
        let children_id = BastionId::new();
        let res = BastionPath::root()
            .append(BastionPathElement::Supervisor(sv_id))
            .unwrap()
            .append(BastionPathElement::Child(children_id));
        assert!(res.is_err())
    }

    // children + smth

    #[test]
    fn append_sv_to_children() {
        let sv_id = BastionId::new();
        let children_id = BastionId::new();
        let res = BastionPath::root()
            .append(BastionPathElement::Supervisor(sv_id))
            .unwrap()
            .append(BastionPathElement::Children(children_id))
            .unwrap()
            .append(BastionPathElement::Supervisor(BastionId::new()));
        assert!(res.is_err())
    }

    #[test]
    fn append_children_to_children() {
        let sv_id = BastionId::new();
        let children_id = BastionId::new();
        let res = BastionPath::root()
            .append(BastionPathElement::Supervisor(sv_id))
            .unwrap()
            .append(BastionPathElement::Children(children_id))
            .unwrap()
            .append(BastionPathElement::Children(BastionId::new()));
        assert!(res.is_err())
    }

    #[test]
    fn append_child_to_children() {
        let sv_id = BastionId::new();
        let children_id = BastionId::new();
        let child_id = BastionId::new();
        let path = BastionPath::root()
            .append(BastionPathElement::Supervisor(sv_id.clone()))
            .unwrap()
            .append(BastionPathElement::Children(children_id.clone()))
            .unwrap()
            .append(BastionPathElement::Child(child_id.clone()))
            .unwrap();
        assert_eq!(
            path.iter().collect::<Vec<&BastionId>>(),
            vec![&sv_id, &children_id, &child_id]
        );
    }

    // child + smth

    #[test]
    fn append_sv_to_child() {
        let sv_id = BastionId::new();
        let children_id = BastionId::new();
        let child_id = BastionId::new();
        let res = BastionPath::root()
            .append(BastionPathElement::Supervisor(sv_id))
            .unwrap()
            .append(BastionPathElement::Children(children_id))
            .unwrap()
            .append(BastionPathElement::Child(child_id))
            .unwrap()
            .append(BastionPathElement::Supervisor(BastionId::new()));
        assert!(res.is_err())
    }

    #[test]
    fn append_children_to_child() {
        let sv_id = BastionId::new();
        let children_id = BastionId::new();
        let child_id = BastionId::new();
        let res = BastionPath::root()
            .append(BastionPathElement::Supervisor(sv_id))
            .unwrap()
            .append(BastionPathElement::Children(children_id))
            .unwrap()
            .append(BastionPathElement::Child(child_id))
            .unwrap()
            .append(BastionPathElement::Children(BastionId::new()));
        assert!(res.is_err())
    }

    #[test]
    fn append_child_to_child() {
        let sv_id = BastionId::new();
        let children_id = BastionId::new();
        let child_id = BastionId::new();
        let res = BastionPath::root()
            .append(BastionPathElement::Supervisor(sv_id))
            .unwrap()
            .append(BastionPathElement::Children(children_id))
            .unwrap()
            .append(BastionPathElement::Child(child_id))
            .unwrap()
            .append(BastionPathElement::Child(BastionId::new()));
        assert!(res.is_err())
    }
}
