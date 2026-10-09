impl Expand {
    fn constraints(
        &mut self,
        owner: usize,
        names: &[String],
        bounds: &[(String, Vec<String>)],
        args: &[Type],
        span: Span,
    ) -> Result<(), String> {
        let bindings = names
            .iter()
            .cloned()
            .zip(args.iter().cloned())
            .collect::<Bindings>();
        for name in names {
            let t = &bindings[name];
            let constant = bounds
                .iter()
                .any(|(n, bs)| n == name && bs.iter().any(|b| b == "const"));
            if constant != matches!(t, Type::Const(_)) {
                return Err(error(
                    &self.paths[owner],
                    span,
                    "generic argument must match type/const parameter kind",
                ));
            }
        }
        for (name, traits) in bounds {
            let t = &bindings[name];
            for bound in traits {
                let builtin = match bound.as_str() {
                    "const" => Some(matches!(t, Type::Const(_))),
                    "Number" => Some(t.numeric()),
                    "Integer" => Some(t.integer()),
                    "Equatable" => Some(
                        t.scalar()
                            || matches!(t,Type::Enum(_,vs) if vs.iter().all(|(_,ts)|ts.is_empty())),
                    ),
                    _ => None,
                };
                let valid = if let Some(v) = builtin {
                    v
                } else {
                    let path = bound.split('.').map(str::to_owned).collect::<Vec<_>>();
                    let (trait_owner, trait_name) = self
                        .target(owner, &path)
                        .ok_or_else(|| error(&self.paths[owner], span, "unknown trait module"))?;
                    let definition = self.traits[trait_owner]
                        .get(&trait_name)
                        .cloned()
                        .ok_or_else(|| {
                            error(&self.paths[owner], span, format!("unknown trait '{bound}'"))
                        })?;
                    let nominal = match t {
                        Type::Record(n, _) | Type::Enum(n, _) | Type::Nominal(n) => Some(n),
                        _ => None,
                    };
                    if let Some((type_owner, type_name, type_args)) =
                        nominal.and_then(|n| self.nominals.get(n)).cloned()
                    {
                        let mut valid = true;
                        for required in definition.methods {
                            let candidate = self.functions[type_owner]
                                .get(&format!("method_{type_name}_{}", required.name))
                                .cloned();
                            if let Some(f) = candidate {
                                let mut method_env = HashMap::new();
                                self.unify(
                                    type_owner,
                                    &f.params
                                        .first()
                                        .map(|(_, t)| t.clone())
                                        .unwrap_or(Type::Void),
                                    t,
                                    &f.generics,
                                    &mut method_env,
                                )?;
                                // The receiver also supplies generic type arguments to methods.
                                for (n, a) in self.defs[type_owner][&type_name]
                                    .generics
                                    .iter()
                                    .zip(&type_args)
                                {
                                    method_env.entry(n.clone()).or_insert_with(|| a.clone());
                                }
                                let actual_ps = f
                                    .params
                                    .iter()
                                    .map(|(_, t)| self.ty(type_owner, t, &method_env, span))
                                    .collect::<Result<Vec<_>, _>>()?;
                                let actual_ret = self.ty(type_owner, &f.ret, &method_env, span)?;
                                let required_env = HashMap::from([("Self".into(), t.clone())]);
                                let expected_ps = required
                                    .params
                                    .iter()
                                    .map(|(_, t)| self.ty(trait_owner, t, &required_env, span))
                                    .collect::<Result<Vec<_>, _>>()?;
                                let expected_ret =
                                    self.ty(trait_owner, &required.ret, &required_env, span)?;
                                valid &= actual_ps == expected_ps && actual_ret == expected_ret;
                            } else {
                                valid = false;
                            }
                        }
                        valid
                    } else {
                        false
                    }
                };
                if !valid {
                    return Err(error(
                        &self.paths[owner],
                        span,
                        format!("{} does not satisfy constraint {bound}", t.name()),
                    ));
                }
            }
        }
        Ok(())
    }
}
