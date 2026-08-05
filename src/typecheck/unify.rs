impl TypeChecker {
    fn unify(&self, a: &Ty, b: &Ty) -> bool {
        Ty::unifyable_with_aliases(a, b, &self.type_aliases)
    }


    fn unify_or_hole(&self, a: &Ty, b: &Ty) -> bool {
        Ty::unifyable_or_unknown_hole_with_aliases(a, b, &self.type_aliases)
    }


    fn norm(&self, t: &Ty) -> Ty {
        t.normalize(&self.type_aliases)
    }


    /// `Int[lo..hi]` or a type alias that expands to one (`type Port = Int[1..10]`).
    fn bounds_from_type_ann(&self, ann: &Type) -> Option<(i64, i64)> {
        int_refinement_bounds(ann).or_else(|| {
            if let Type::Custom(name) = ann {
                self.alias_refinements.get(name).copied()
            } else {
                None
            }
        })
    }

}
