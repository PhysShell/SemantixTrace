//! The [`Upcaster`] and [`StreamUpcaster`] traits.
//!
//! These traits are **sealed** (ADR-0012 `C-SEALED`): only this crate's
//! own modules may implement them. The version-dispatch invariant in
//! [`crate::read_event`] depends on the closed set; a third-party impl
//! could lie about its `From` / `To` association and break readers.
//!
//! S1 ships an identity chain: `Current = v1::TraceEvent`, no upcasters
//! defined yet beyond the trivial blanket. Real upcasters land at the
//! first schema bump per the procedure in `docs/upcasters.md`.

/// Crate-private sealing module. Downstream crates cannot implement
/// [`Upcaster`] or [`StreamUpcaster`] because they cannot name
/// `sealed::Sealed`.
pub mod sealed {
    /// Sealing marker. Downstream crates cannot implement this trait,
    /// which prevents them from implementing [`super::Upcaster`] or
    /// [`super::StreamUpcaster`] (ADR-0012 `C-SEALED`).
    pub trait Sealed {}
}

/// Upcast a single event from version `N` to version `N + 1`.
///
/// Implementations live alongside the destination module (`v2`'s
/// `From<v1::TraceEvent> for v2::TraceEvent`); the trait itself is
/// sealed so the set of upcasters is fixed by `trace-schema`.
pub trait Upcaster: sealed::Sealed {
    /// The older event shape.
    type From;
    /// The newer event shape.
    type To;

    /// Whether this upcast drops information that cannot be recovered.
    ///
    /// A `Current` event produced through a chain with `LOSSY = true`
    /// is **not** round-trippable to its original version. Plans /
    /// reporters that surface "degraded historical event" markers read
    /// this constant.
    const LOSSY: bool;

    /// Lift the event from the older shape to the newer one.
    fn upcast(input: Self::From) -> Self::To;
}

/// Upcast a stream of events from version `N` to version `N + 1`.
///
/// Used when an upcast cannot be one-to-one (e.g. v3 collapses two
/// adjacent v2 events into one v3 event, or v4 splits one v3 event
/// into two v4 events). Axon Framework draws the same distinction
/// between `SingleEventUpcaster` and `EventUpcaster`; we mirror it for
/// the same reason.
pub trait StreamUpcaster: sealed::Sealed {
    /// The older event shape.
    type From;
    /// The newer event shape.
    type To;

    /// Whether this upcast drops information that cannot be recovered.
    const LOSSY: bool;

    /// Lift a stream of older events to a stream of newer events.
    fn upcast_stream<'a, I>(input: I) -> Box<dyn Iterator<Item = Self::To> + 'a>
    where
        I: Iterator<Item = Self::From> + 'a,
        Self::To: 'a;
}
