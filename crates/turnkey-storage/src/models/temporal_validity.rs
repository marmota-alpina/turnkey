//! Trait for entities with temporal validity periods
//!
//! This module provides a reusable trait for domain entities that have
//! activation status and time-based validity periods. This eliminates
//! code duplication between User and Card models.
//!
//! # Design Pattern
//!
//! This trait implements the **Template Method** pattern, where the validation
//! algorithm (`is_valid()`) is defined once, but implementors provide the
//! specific data access methods (`is_active()`, `validity_start()`, `validity_end()`).
//!
//! # Usage
//!
//! To add temporal validity to a new entity:
//!
//! ```
//! use turnkey_storage::models::TemporalValidity;
//! use chrono::{DateTime, Utc};
//!
//! struct MyEntity {
//!     active: bool,
//!     start_date: Option<DateTime<Utc>>,
//!     end_date: Option<DateTime<Utc>>,
//! }
//!
//! impl TemporalValidity for MyEntity {
//!     fn is_active(&self) -> bool {
//!         self.active
//!     }
//!
//!     fn validity_start(&self) -> Option<DateTime<Utc>> {
//!         self.start_date
//!     }
//!
//!     fn validity_end(&self) -> Option<DateTime<Utc>> {
//!         self.end_date
//!     }
//! }
//!
//! # fn main() {
//! # let entity = MyEntity { active: true, start_date: None, end_date: None };
//! // Now `is_valid()` method is automatically available
//! assert!(entity.is_valid());
//! # }
//! ```

use chrono::{DateTime, Utc};

/// Trait for entities with temporal validity periods
///
/// This trait encapsulates the common validation logic for entities that have:
/// - An active/inactive status
/// - Optional validity start date
/// - Optional validity end date
///
/// # Examples
///
/// ```
/// use turnkey_storage::models::{User, Card, TemporalValidity};
/// use chrono::Utc;
///
/// # fn example(user: User, card: Card) {
/// // Both User and Card implement TemporalValidity
/// if user.is_valid() {
///     println!("User is currently valid");
/// }
///
/// if card.is_valid() {
///     println!("Card is currently valid");
/// }
/// # }
/// ```
pub trait TemporalValidity {
    /// Returns whether the entity is currently active
    ///
    /// Inactive entities are always considered invalid regardless of date ranges.
    fn is_active(&self) -> bool;

    /// Returns the validity start date (if any)
    ///
    /// If set, the entity cannot be used before this date.
    fn validity_start(&self) -> Option<DateTime<Utc>>;

    /// Returns the validity end date (if any)
    ///
    /// If set, the entity cannot be used after this date.
    fn validity_end(&self) -> Option<DateTime<Utc>>;

    /// Check if the entity is currently valid based on validity period
    ///
    /// Returns `true` if all of the following conditions are met:
    /// - Entity is active (`is_active()` returns true)
    /// - Current time is after validity start (or no start date is set)
    /// - Current time is before validity end (or no end date is set)
    ///
    /// # Implementation Note
    ///
    /// This method has a default implementation that uses the other trait methods.
    /// Implementors only need to provide `is_active()`, `validity_start()`, and
    /// `validity_end()` - the validation logic is automatically provided.
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_storage::models::{User, TemporalValidity};
    /// # use chrono::Utc;
    ///
    /// # fn example(user: User) {
    /// if user.is_valid() {
    ///     println!("User can access the system");
    /// } else {
    ///     println!("Access denied - user is invalid or expired");
    /// }
    /// # }
    /// ```
    fn is_valid(&self) -> bool {
        // Entity must be active
        if !self.is_active() {
            return false;
        }

        let now = Utc::now();

        // Check validity start
        if let Some(start) = self.validity_start()
            && now < start
        {
            return false;
        }

        // Check validity end
        if let Some(end) = self.validity_end()
            && now > end
        {
            return false;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock entity for testing the trait
    struct MockEntity {
        active: bool,
        start: Option<DateTime<Utc>>,
        end: Option<DateTime<Utc>>,
    }

    impl TemporalValidity for MockEntity {
        fn is_active(&self) -> bool {
            self.active
        }

        fn validity_start(&self) -> Option<DateTime<Utc>> {
            self.start
        }

        fn validity_end(&self) -> Option<DateTime<Utc>> {
            self.end
        }
    }

    #[test]
    fn test_active_entity_with_no_dates_is_valid() {
        let entity = MockEntity {
            active: true,
            start: None,
            end: None,
        };

        assert!(entity.is_valid());
    }

    #[test]
    fn test_inactive_entity_is_invalid() {
        let entity = MockEntity {
            active: false,
            start: None,
            end: None,
        };

        assert!(!entity.is_valid());
    }

    #[test]
    fn test_entity_before_start_date_is_invalid() {
        use chrono::Duration;

        let entity = MockEntity {
            active: true,
            start: Some(Utc::now() + Duration::days(1)), // Future start date
            end: None,
        };

        assert!(!entity.is_valid());
    }

    #[test]
    fn test_entity_after_end_date_is_invalid() {
        use chrono::Duration;

        let entity = MockEntity {
            active: true,
            start: None,
            end: Some(Utc::now() - Duration::days(1)), // Past end date
        };

        assert!(!entity.is_valid());
    }

    #[test]
    fn test_entity_within_validity_period_is_valid() {
        use chrono::Duration;

        let entity = MockEntity {
            active: true,
            start: Some(Utc::now() - Duration::days(1)), // Started yesterday
            end: Some(Utc::now() + Duration::days(30)),  // Ends in 30 days
        };

        assert!(entity.is_valid());
    }
}
