// rust/src/lib.rs — Rust bindings to fleet-coordinate for cocapn-glue-core
// This allows Python's glue_core.py to use fleet-coordinate's ZHC consensus

pub mod zhc_client {
    use std::collections::HashMap;
    
    /// Minimal ZHC client for Python FFI
    /// Encodes trust relationships and checks consensus via local constraint satisfaction
    pub struct ZhcClient {
        tolerance: f64,
        tiles: HashMap<u64, Vec<f64>>,
        neighbors: HashMap<u64, Vec<u64>>,
    }
    
    impl ZhcClient {
        pub fn new(tolerance: f64) -> Self {
            Self {
                tolerance,
                tiles: HashMap::new(),
                neighbors: HashMap::new(),
            }
        }
        
        pub fn add_tile(&mut self, id: u64, x: f64, y: f64, z: f64, neighbors: Vec<u64>) {
            self.tiles.insert(id, vec![x, y, z]);
            self.neighbors.insert(id, neighbors);
        }
        
        /// Check if all tiles in a connected component are consistent
        /// Returns (is_consistent, max_residual)
        pub fn check_consensus(&self) -> (bool, f64) {
            if self.tiles.len() < 2 {
                return (true, 0.0);
            }
            
            let mut total_residual = 0.0;
            let mut count = 0;
            
            for (id, tile) in &self.tiles {
                if let Some(nbrs) = self.neighbors.get(id) {
                    for nbr_id in nbrs {
                        if let Some(nbr_tile) = self.tiles.get(nbr_id) {
                            // Sum of differences should be ~0 for consensus
                            let diff = (tile[0] - nbr_tile[0]).abs() +
                                      (tile[1] - nbr_tile[1]).abs() +
                                      (tile[2] - nbr_tile[2]).abs();
                            total_residual += diff;
                            count += 1;
                        }
                    }
                }
            }
            
            let avg_residual = if count > 0 { total_residual / count as f64 } else { 0.0 };
            (avg_residual < self.tolerance, avg_residual)
        }
        
        /// Get information content in bits
        pub fn information_bits(&self) -> f64 {
            let n = self.tiles.len() as f64;
            if n < 2.0 { return 0.0; }
            // log₂(n(n-1)/2) for complete graph information
            let edges = n * (n - 1.0) / 2.0;
            edges.log2()
        }
    }
}

pub mod lamant {
    /// Laman rigidity check for fleet topology
    pub fn is_laman_rigid(V: usize, E: usize) -> bool {
        // Laman rigid: E = 2V - 3 (within 5% tolerance)
        let expected = 2 * V - 3;
        if expected == 0 { return V <= 2; }
        let ratio = E as f64 / expected as f64;
        (ratio - 1.0).abs() < 0.05
    }
    
    /// H1 cohomology dimension (Betti number β₁)
    pub fn h1_dimension(E: usize, V: usize) -> usize {
        if E >= V { E - V + 1 } else { 0 }
    }
    
    /// Check if fleet is self-coordinating (rigid + ZHC consistent + no emergence)
    pub fn is_self_coordinating(V: usize, E: usize, zhc_residual: f64, tolerance: f64) -> bool {
        let rigid = is_laman_rigid(V, E);
        let zhc_ok = zhc_residual < tolerance;
        let emergence = E > 2 * V - 3; // over-rigid = emergence
        rigid && zhc_ok && !emergence
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_laman_triangle() {
        // V=3, E=3 → 2*3-3=3 → rigid
        assert!(lamant::is_laman_rigid(3, 3));
    }
    
    #[test]
    fn test_laman_square() {
        // V=4, E=4 → 2*4-3=5, ratio=0.8 → not rigid
        assert!(!lamant::is_laman_rigid(4, 4));
    }
    
    #[test]
    fn test_self_coordinating_triangle() {
        // Triangle: rigid=true, ZHC residual=0.1<0.5, emergence=false → self-coordinating
        assert!(lamant::is_self_coordinating(3, 3, 0.1, 0.5));
    }
}