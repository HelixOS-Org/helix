    //! Auto-Attack (ensemble of attacks) implementation.

    use alloc::vec::Vec;

    use crate::adversarial::perturbation::Perturbation;
    use crate::adversarial::pgd::PGD;
    use crate::adversarial::types::PerturbationType;
    use crate::adversarial::utils::lcg_next;

    /// Auto-Attack (ensemble of attacks)
    #[derive(Debug, Clone)]
    pub struct AutoAttack {
        /// Epsilon bound
        pub epsilon: f64,
        /// Perturbation type
        pub pert_type: PerturbationType,
        /// Use APGD-CE
        pub use_apgd_ce: bool,
        /// Use APGD-DLR
        pub use_apgd_dlr: bool,
        /// Use FAB
        pub use_fab: bool,
        /// Use Square attack
        pub use_square: bool,
    }

    impl AutoAttack {
        /// Create a new AutoAttack
        pub fn new(epsilon: f64) -> Self {
            Self {
                epsilon,
                pert_type: PerturbationType::LInf,
                use_apgd_ce: true,
                use_apgd_dlr: true,
                use_fab: false,
                use_square: true,
            }
        }

        /// Run attack
        pub fn attack<F>(&self, input: &[f64], grad_fn: F, seed: u64) -> Perturbation
        where
            F: Fn(&[f64]) -> Vec<f64>,
        {
            let dim = input.len();
            let mut best_perturbation = Perturbation::new(dim, self.pert_type, self.epsilon);
            let mut best_norm = f64::INFINITY;

            // Run PGD with different settings
            if self.use_apgd_ce {
                let pgd = PGD::new(self.epsilon, self.epsilon / 4.0, 100);
                let pert = pgd.attack(input, |x| grad_fn(x), seed);

                if pert.l2_norm() < best_norm {
                    best_norm = pert.l2_norm();
                    best_perturbation = pert;
                }
            }

            if self.use_square {
                // Square attack (query-based)
                let pert = self.square_attack(input, seed);

                if pert.l2_norm() < best_norm && pert.success {
                    best_perturbation = pert;
                }
            }

            best_perturbation
        }

        /// Square attack (simplified)
        fn square_attack(&self, input: &[f64], seed: u64) -> Perturbation {
            let dim = input.len();
            let mut perturbation = Perturbation::new(dim, self.pert_type, self.epsilon);

            let mut rng = seed;

            // Initialize with random corners
            for d in &mut perturbation.delta {
                rng = lcg_next(rng);
                *d = if rng % 2 == 0 {
                    self.epsilon
                } else {
                    -self.epsilon
                };
            }

            perturbation
        }
    }
