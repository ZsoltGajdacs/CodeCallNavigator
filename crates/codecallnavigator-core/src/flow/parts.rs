//! One typed child walk. Ordinary validation allocates no path nodes; located
//! traversal retains a compact parent arena and materializes only requested paths.
use super::{
    CallSite, Expression, FunctionFlowIR, IrField, IrPath, IrPathComponent, SourceSpan, Statement,
    StatementKind, SwitchCase,
};
const ROOT: usize = usize::MAX;
#[derive(Clone, Copy)]
pub(super) enum Owner<'a> {
    Action,
    Value(&'a Expression),
}
#[derive(Clone, Copy)]
pub(super) enum Part<'a> {
    Statement(&'a Statement),
    Expression(&'a Expression),
    Case(&'a SwitchCase),
    Call {
        site: &'a CallSite,
        owner: Owner<'a>,
    },
}
pub(super) struct LocatedPart<'a> {
    pub part: Part<'a>,
    pub path: usize,
}
struct ChildLocation {
    field: IrField,
    index: Option<usize>,
}
impl ChildLocation {
    fn field(field: IrField) -> Self {
        Self { field, index: None }
    }
    fn element(field: IrField, index: usize) -> Self {
        Self {
            field,
            index: Some(index),
        }
    }
}
struct PathNode {
    parent: usize,
    component: IrPathComponent,
}
pub(super) struct Parts<'a> {
    pending: Vec<LocatedPart<'a>>,
    paths: Vec<PathNode>,
    located: bool,
}
impl<'a> Parts<'a> {
    pub fn new(function: &'a FunctionFlowIR) -> Self {
        Self::start(function, false)
    }
    pub fn located(function: &'a FunctionFlowIR) -> Self {
        Self::start(function, true)
    }
    fn start(function: &'a FunctionFlowIR, located: bool) -> Self {
        let mut parts = Self {
            pending: Vec::new(),
            paths: Vec::new(),
            located,
        };
        parts.statements(ROOT, IrField::Body, &function.body);
        parts
    }
    fn child(&mut self, parent: usize, field: IrField, index: Option<usize>) -> usize {
        if !self.located {
            return ROOT;
        }
        let field_path = self.record(parent, IrPathComponent::Field { field });
        match index {
            Some(index) => self.record(field_path, IrPathComponent::Index { index }),
            None => field_path,
        }
    }
    fn record(&mut self, parent: usize, component: IrPathComponent) -> usize {
        let id = self.paths.len();
        self.paths.push(PathNode { parent, component });
        id
    }
    pub fn path(&self, mut id: usize) -> IrPath {
        let mut components = Vec::new();
        while id != ROOT {
            let node = &self.paths[id];
            components.push(node.component);
            id = node.parent;
        }
        components.reverse();
        IrPath(components)
    }
    fn push(&mut self, part: Part<'a>, parent: usize, location: ChildLocation) {
        let path = self.child(parent, location.field, location.index);
        self.pending.push(LocatedPart { part, path });
    }
    fn statements(&mut self, parent: usize, field: IrField, body: &'a [Statement]) {
        for (index, node) in body.iter().enumerate() {
            self.push(
                Part::Statement(node),
                parent,
                ChildLocation::element(field, index),
            );
        }
    }
    fn expressions(&mut self, parent: usize, field: IrField, items: &'a [Expression]) {
        for (index, node) in items.iter().enumerate() {
            self.push(
                Part::Expression(node),
                parent,
                ChildLocation::element(field, index),
            );
        }
    }
    fn optional(&mut self, parent: usize, field: IrField, node: Option<&'a Expression>) {
        if let Some(node) = node {
            self.push(Part::Expression(node), parent, ChildLocation::field(field));
        }
    }
    fn calls(&mut self, parent: usize, sites: &'a [CallSite], owner: Owner<'a>) {
        for (index, site) in sites.iter().enumerate() {
            self.push(
                Part::Call { site, owner },
                parent,
                ChildLocation::element(IrField::Calls, index),
            );
        }
    }
    fn cases(&mut self, parent: usize, cases: &'a [SwitchCase]) {
        for (index, node) in cases.iter().enumerate() {
            self.push(
                Part::Case(node),
                parent,
                ChildLocation::element(IrField::Cases, index),
            );
        }
    }
    pub fn next_located(&mut self) -> Option<LocatedPart<'a>> {
        let next = self.pending.pop()?;
        match next.part {
            Part::Statement(node) => self.statement(node, next.path),
            Part::Expression(node) => self.expression(node, next.path),
            Part::Case(node) => {
                self.optional(next.path, IrField::Test, node.test.as_ref());
                self.statements(next.path, IrField::Body, &node.body);
            }
            Part::Call { .. } => (),
        }
        Some(next)
    }
    fn statement(&mut self, node: &'a Statement, path: usize) {
        use IrField as F;
        match &node.kind {
            StatementKind::Action { calls, .. } => self.calls(path, calls, Owner::Action),
            StatementKind::Evaluate { value } | StatementKind::Throw { value } => {
                self.optional(path, F::Value, Some(value))
            }
            StatementKind::Return { value } | StatementKind::Yield { value } => {
                self.optional(path, F::Value, value.as_ref())
            }
            StatementKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.optional(path, F::Condition, Some(condition));
                self.statements(path, F::ThenBranch, then_branch);
                self.statements(path, F::ElseBranch, else_branch);
            }
            StatementKind::Loop {
                condition,
                body,
                update,
                ..
            } => {
                self.optional(path, F::Condition, condition.as_ref());
                self.statements(path, F::Body, body);
                self.statements(path, F::Update, update);
            }
            StatementKind::Switch { subject, cases, .. } => {
                self.optional(path, F::Subject, Some(subject));
                self.cases(path, cases);
            }
            StatementKind::Try {
                resources,
                body,
                catches,
                finally,
                cleanup,
            } => {
                self.statements(path, F::Resources, resources);
                self.statements(path, F::Body, body);
                self.statements(path, F::Finally, finally);
                self.statements(path, F::Cleanup, cleanup);
                for (index, catch) in catches.iter().enumerate() {
                    let parent = self.child(path, F::Catches, Some(index));
                    self.statements(parent, F::Body, &catch.body);
                }
            }
            StatementKind::Block { body, .. } | StatementKind::Region { body, .. } => {
                self.statements(path, F::Body, body)
            }
            _ => (),
        }
    }
    fn expression(&mut self, node: &'a Expression, path: usize) {
        use IrField as F;
        match node {
            Expression::Sequence { items, .. } => self.expressions(path, F::Items, items),
            Expression::Value { calls, regions, .. } => {
                self.calls(path, calls, Owner::Value(node));
                self.statements(path, F::Regions, regions);
            }
            Expression::And { left, right, .. } | Expression::Or { left, right, .. } => {
                self.optional(path, F::Left, Some(left));
                self.optional(path, F::Right, Some(right));
            }
            Expression::Conditional {
                condition,
                when_true,
                when_false,
                ..
            } => {
                self.optional(path, F::Condition, Some(condition));
                self.optional(path, F::WhenTrue, Some(when_true));
                self.optional(path, F::WhenFalse, Some(when_false));
            }
            Expression::Switch { subject, cases, .. } => {
                self.optional(path, F::Subject, Some(subject));
                self.cases(path, cases);
            }
        }
    }
}
impl<'a> Iterator for Parts<'a> {
    type Item = Part<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        self.next_located().map(|part| part.part)
    }
}
impl Part<'_> {
    pub fn span(&self) -> SourceSpan {
        match self {
            Self::Statement(node) => node.span,
            Self::Expression(node) => node.span(),
            Self::Case(node) => node.span,
            Self::Call { site, .. } => site.span,
        }
    }
}
