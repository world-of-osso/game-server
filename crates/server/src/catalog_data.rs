//! Collection backing: production owns T unchanged; tests share immutable T
//! until an App explicitly mutates or consumes its independent resource copy.
#[cfg(not(test))]
pub type CatalogData<T> = T;

#[cfg(test)]
pub use shared::CatalogData;

#[cfg(test)]
mod shared {
    use std::fmt;
    use std::ops::{Deref, DerefMut};
    use std::sync::Arc;

    #[derive(Default, PartialEq, Eq)]
    pub struct CatalogData<T>(Arc<T>);

    impl<T> Clone for CatalogData<T> {
        fn clone(&self) -> Self {
            Self(self.0.clone())
        }
    }

    impl<T: fmt::Debug> fmt::Debug for CatalogData<T> {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            self.0.as_ref().fmt(formatter)
        }
    }

    impl<T> From<T> for CatalogData<T> {
        fn from(value: T) -> Self {
            Self(Arc::new(value))
        }
    }

    impl<T> Deref for CatalogData<T> {
        type Target = T;

        fn deref(&self) -> &T {
            &self.0
        }
    }

    impl<T: Clone> DerefMut for CatalogData<T> {
        fn deref_mut(&mut self) -> &mut T {
            Arc::make_mut(&mut self.0)
        }
    }

    impl<T: Clone> CatalogData<T> {
        pub fn into_owned(self) -> T {
            Arc::unwrap_or_clone(self.0)
        }
    }

    impl<T: Clone + IntoIterator> IntoIterator for CatalogData<T> {
        type Item = T::Item;
        type IntoIter = T::IntoIter;

        fn into_iter(self) -> Self::IntoIter {
            self.into_owned().into_iter()
        }
    }

    impl<'a, T> IntoIterator for &'a CatalogData<T>
    where
        &'a T: IntoIterator,
    {
        type Item = <&'a T as IntoIterator>::Item;
        type IntoIter = <&'a T as IntoIterator>::IntoIter;

        fn into_iter(self) -> Self::IntoIter {
            self.0.as_ref().into_iter()
        }
    }

    impl<'a, T: Clone> IntoIterator for &'a mut CatalogData<T>
    where
        &'a mut T: IntoIterator,
    {
        type Item = <&'a mut T as IntoIterator>::Item;
        type IntoIter = <&'a mut T as IntoIterator>::IntoIter;

        fn into_iter(self) -> Self::IntoIter {
            (&mut **self).into_iter()
        }
    }

    impl<A, T: FromIterator<A>> FromIterator<A> for CatalogData<T> {
        fn from_iter<I: IntoIterator<Item = A>>(iter: I) -> Self {
            Self::from(T::from_iter(iter))
        }
    }

    impl<T: serde::Serialize> serde::Serialize for CatalogData<T> {
        fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            self.0.as_ref().serialize(serializer)
        }
    }
}
