#![allow(dead_code)]

use std::{any::Any, cell::{Cell, RefCell}, collections::{HashMap, HashSet}, ops::{Deref, DerefMut}};

/// Represents collections that track changes to their internal contents
pub trait DirtyTracker {
    fn is_dirty(&self, idx: usize) -> bool;
    fn mark_dirty(&self, idx: usize);
    fn mark_clean(&self, idx: usize);
}

/// Represents collections that can push to their internal contents
pub trait Pushable {
    type Item: Clone + Default + 'static;

    fn push(&mut self, data: Self::Item);
}

/// Represents a type erased vector that stores generic data
pub trait DynVec: Any {
    /// Push a default value into the internal vector
    fn push_default(&mut self);
    /// Swap the last element in the vector with the one at the index, and then remove the last element.
    fn swap_remove(&mut self, idx: usize);
    /// Clone the values of the vector into a new one.
    fn clone_box(&self) -> Box<dyn DynVec>;
    /// Create a new empty vector
    fn clone_empty(&self) -> Box<dyn DynVec>;
    /// Append data from another vector into this one.
    fn append_from(&mut self, other: &dyn DynVec);
    /// Get the length of the vector
    fn len(&self) -> usize;
    /// clear the dyn vector
    fn clear(&mut self);

    /// Convert the DynVec into a reference of it's Any type
    fn as_any_ref<'a>(&'a self) -> &'a dyn Any;
    /// Convert the data store into it's Any type as mutable
    fn as_any_mut<'a>(&'a mut self) -> &'a mut dyn Any;

    /// Attempt to cast this DynVec into a DirtyTracker.
    fn as_dirty_tracker(&self) -> Option<&dyn DirtyTracker> { None }
}

impl dyn DynVec {
    /// Downcast this dynamic vector into a reference of the concrete vector type
    pub fn downcast_ref<V: DynVec + 'static>(&self) -> Option<&V> {
        self.as_any_ref().downcast_ref::<V>()
    }

    /// Downcast this dynamic vector into a mutable reference of the concrete vector type
    pub fn downcast_mut<V: DynVec + 'static>(&mut self) -> Option<&mut V> {
        self.as_any_mut().downcast_mut::<V>()
    }
}

impl<T: Clone + Default + 'static> DynVec for Vec<T> {
    fn as_any_mut<'a>(&'a mut self) -> &'a mut dyn Any { self }
    fn as_any_ref<'a>(&'a self) -> &'a dyn Any { self }

    fn swap_remove(&mut self, idx: usize) { self.swap_remove(idx); }

    fn push_default(&mut self) { self.push(T::default()); }

    fn clone_box(&self) -> Box<dyn DynVec> { Box::new(self.clone()) }

    fn clone_empty(&self) -> Box<dyn DynVec> { Box::new(Vec::<T>::new()) }

    fn len(&self) -> usize { self.len() }

    fn clear(&mut self) { self.clear() }

    fn append_from(&mut self, other: &dyn DynVec) {
        if let Some(other_vec) = other.as_any_ref().downcast_ref::<Vec<T>>() {
            for source in other_vec.iter() {
                self.push(source.clone());
            }
        } else {
            panic!("Expected 'other' to contain values with the same type as 'self'");
        }
    }
}

impl<T: Clone + Default + 'static> Pushable for Vec<T> {
    type Item = T;

    fn push(&mut self, data: T) {
        self.push(data);
    }
}

/// A transparent wrapper for frequently updated data with dirty flag handling
#[derive(Debug, Default)]
pub struct DataView<T> {
    data: T,
    is_dirty: Cell<bool>
}

impl<T: Clone> DataView<T> {
    pub fn new(data: T) -> Self {
        Self { data, is_dirty: Cell::new(true) }
    }

    /// Check if the data has changed
    pub fn is_dirty(&self) -> bool {
        self.is_dirty.get()
    }

    /// Mark the data as clean
    pub fn mark_clean(&self) {
        self.is_dirty.set(false);
    }

    /// Mark the data as dirty
    pub fn mark_dirty(&self) {
        self.is_dirty.set(true);
    }

    /// Set the data held in the view, marking it dirty
    pub fn set(&mut self, data: T) {
        self.data = data;
        self.is_dirty.set(true);
    }
}

impl<T: Clone> Clone for DataView<T> {
    fn clone(&self) -> Self {
        DataView::new(self.data.clone())
    }
}

impl<T> Deref for DataView<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target { &self.data }
}

impl<T> DerefMut for DataView<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.is_dirty.set(true);
        &mut self.data
    }
}

#[derive(Debug, Clone)]
pub struct DirtyVec<T> {
    inner: Vec<DataView<T>>,
}

impl<T: Default + Clone + 'static> DirtyTracker for DirtyVec<T> {
    fn is_dirty(&self, idx: usize) -> bool {
        self.inner[idx].is_dirty()
    }

    fn mark_dirty(&self, idx: usize) {
        self.inner[idx].mark_dirty();
    }

    fn mark_clean(&self, idx: usize) {
        self.inner[idx].mark_clean();
    }
}

impl<T: Clone + Default + 'static> Pushable for DirtyVec<T> {
    type Item = T;

    fn push(&mut self, data: T) {
        self.inner.push(DataView::new(data));
    }
}

impl<T: Default + Clone + 'static> DynVec for DirtyVec<T> {
    fn push_default(&mut self) {
        self.inner.push(DataView::new(T::default()));
    }

    fn append_from(&mut self, other: &dyn DynVec) {
        if let Some(other_vec) = other.as_any_ref().downcast_ref::<DirtyVec<T>>() {
            for source in other_vec.inner.iter() {
                self.inner.push(source.clone());
            }
        } else {
            panic!("Expected 'other' to contain values with the same type as 'self'");
        }
    }

    fn clone_box(&self) -> Box<dyn DynVec> {
        Box::new(Self { inner: self.inner.clone() })
    }

    fn clone_empty(&self) -> Box<dyn DynVec> {
        Box::new(Self { inner: Vec::<DataView<T>>::new() })
    }

    fn swap_remove(&mut self, idx: usize) {
        if idx >= self.inner.len() { return; }

        self.inner.swap_remove(idx);

        // handle case that idx pointed to the last element,
        // in which case there is nothing to mark dirty.
        if idx < self.inner.len() {
            self.inner[idx].mark_dirty();
        }
    }

    fn len(&self) -> usize {
        self.inner.len()
    }

    fn clear(&mut self) {
        self.inner.clear();
    }

    fn as_dirty_tracker(&self) -> Option<&dyn DirtyTracker> {
        Some(self as &dyn DirtyTracker)
    }

    fn as_any_ref<'a>(&'a self) -> &'a dyn Any { self }
    fn as_any_mut<'a>(&'a mut self) -> &'a mut dyn Any { self }
}

impl<T: Default + Clone + 'static> DirtyVec<T> {
    /// Create a new DirtyVec with an initial capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self { inner: Vec::<DataView<T>>::with_capacity(capacity) }
    }

    /// Move all elements from other into self, leaving other empty.
    pub fn append(&mut self, other: Vec<T>) {
        for elem in other.into_iter() {
            self.inner.push(DataView::new(elem));
        }
    }

    /// Create a dirty vector from a Vec of type T
    pub fn from_vec(data_vec: Vec<T>) -> Self {
        let dirty_vec = data_vec.into_iter()
            .map(|data| DataView::new(data))
            .collect();

        Self { inner: dirty_vec }
    }

    /// Get a reference to a raw element in the internal Vec
    pub fn get_raw(&self, idx: usize) -> Option<&T> {
        Some(&*self.inner.get(idx)?)
    }
}

impl<T: Clone + Default + 'static> Deref for DirtyVec<T> {
    type Target = Vec<DataView<T>>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T: Clone + Default + 'static> DerefMut for DirtyVec<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

/// Allows mutable access to a property in a DataTable
pub struct PropertyViewMut<'a, V: DynVec> {
    value: &'a mut V,
    key: String,
    accessed: &'a RefCell<HashSet<String>>,
}

impl<'a, V: DynVec> Deref for PropertyViewMut<'a, V> {
    type Target = V;
    fn deref(&self) -> &Self::Target { self.value }
}

impl<'a, V: DynVec> DerefMut for PropertyViewMut<'a, V> {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.value }
}

impl<'a, V: DynVec> Drop for PropertyViewMut<'a, V> {
    fn drop(&mut self) {
        self.accessed.borrow_mut().remove(&self.key);
    }
}

/// Allows for mutable access to properties in a DataTable, hiding the raw unsafe memory management for easy data processing
/// 
/// Note: While a DataTableProxy is active, the property vectors in the underlying table are locked in place. It is therefore 
/// recommended to treat an instance of DataTableProxy as transient, not a long-lived representative of the owning struct.
/// 
/// If you need to change the size of the underlying map, use the parent table instead while no proxies are active.
pub struct DataTableProxy<'a> {
    /// The raw pointer to the underlying map in the parent data table.
    table_ptr: *mut HashMap<String, Box<dyn DynVec>>,
    /// The set of rows currently mutably borrowed. This is used to make sure a row is only borrowed once at a given time.
    accessed: RefCell<HashSet<String>>,
    /// Marker to ensure the underlying table remains alive for as long as the proxy does.
    _lifetime_marker: std::marker::PhantomData<&'a mut DataTable>
}

impl<'a> DataTableProxy<'a> {
    pub fn new(table: &mut DataTable) -> Self {
        Self {
            table_ptr: &mut table.properties as *mut HashMap<String, Box<dyn DynVec>>,
            accessed: RefCell::new(HashSet::new()),
            _lifetime_marker: std::marker::PhantomData,
        }
    }
    
    /// Get a reference to the concrete data associated with the provided key, if exists
    pub fn get_property<V: DynVec + 'static>(&'a self, key: &str) -> Option<&'a V> {
        let table = self.get_table_ref();
        table.get(key)?.downcast_ref::<V>()
    }

    /// Get a mutable reference to the concrete data associated with the provided key, if exists.
    /// 
    /// Note: Only the first call to this will result in a row for a given key.
    /// Subsequent calls will result in a None until any previous borrows of that row are dropped.
    pub fn get_property_mut<V: DynVec + 'static>(&'a self, key: &str) -> Option<PropertyViewMut<'a, V>> {
        // prevent double borrowing
        if !self.accessed.borrow_mut().insert(key.to_string()) {
            return None;
        }

        let table = self.get_table_mut();
        let property = table.get_mut(key)?.downcast_mut::<V>()?;

        Some(PropertyViewMut::<'a, V>{
            value: &mut *property,
            key: key.to_string(),
            accessed: &self.accessed
        })
    }

    /// swap remove an element's properties in the table at the specified index
    pub fn swap_remove_all(&self, idx: usize) {
        let table = self.get_table_mut();

        for dynvec in table.values_mut() {
            dynvec.as_mut().swap_remove(idx);
        }
    }

    /// Cast the internal pointer to a reference.
    fn get_table_ref(&self) -> &HashMap<String, Box<dyn DynVec + 'static>> {
        unsafe { &*self.table_ptr }
    }

    /// Cast the internal pointer to a mutable reference.
    fn get_table_mut(&self) -> &mut HashMap<String, Box<dyn DynVec + 'static>> {
        unsafe { &mut *self.table_ptr }
    }
}

/// A homogenous collection of generic data stored in DynVectors keyed by a string.
pub struct DataTable {
    pub properties: HashMap<String, Box<dyn DynVec>>,
    pub capacity: usize,
}

impl DataTable {
    pub fn new(capacity: usize) -> Self {
        Self {
            properties: HashMap::new(),
            capacity,
        }
    }

    /// Get the keys to the properties in this DataTable
    pub fn keys(&self) -> Vec<&String> {
        self.properties.keys()
            .map(|key| key)
            .collect()
    }

    /// Add a property to this data table.
    /// 
    /// * 'key' a unique string referencing the property in the table.
    /// * 'prop_func' a closure that takes in the capacity of the table and returns a DynVec implementation.
    pub fn with_property<V>(mut self, key: &str, prop_func: impl FnOnce(usize) -> V) -> Self 
    where V: DynVec + 'static
    {
        self.add_property(key, prop_func);
        self
    }

    /// Add a property to this data table.
    /// 
    /// * 'key' a unique string referencing the property in the table.
    /// * 'prop_func' a closure that takes in the capacity of the table and returns a DynVec implementation.
    pub fn add_property<V>(&mut self, key: &str, prop_func: impl FnOnce(usize) -> V) 
    where V: DynVec + 'static
    {
        if !self.properties.contains_key(key) {
            let property = prop_func(self.capacity);
            self.properties.insert(key.to_string(), Box::new(property));
        }
    }

    /// Get a reference to the concrete data associated with the provided key, if exists
    pub fn get_property<V: DynVec + 'static>(&self, key: &str) -> Option<&V> {
        Some(self.properties.get(key)?.downcast_ref::<V>()?)
    }

    /// Get a mutable reference to the concrete data associated with the provided key, if exists
    pub fn get_property_mut<V: DynVec + 'static>(&mut self, key: &str) -> Option<&mut V> {
        Some(self.properties.get_mut(key)?.downcast_mut::<V>()?)
    }

    /// Get a mutable proxy for the table. This is useful in cases where multiple rows need to be mutably borrowed.
    /// 
    /// Note: Only one proxy can be created at a time. 
    pub fn borrow_mut<'a>(&'a mut self) -> DataTableProxy<'a> {
        DataTableProxy::new(self)
    }

    /// Append data to an existing property in the table.
    pub fn push_to<V>(&mut self, key: &str, data: V::Item) -> Result<(), String> 
    where V: DynVec + Pushable + 'static
    {
        let capacity = self.capacity;
        if let Some(property) = self.get_property_mut::<V>(key) {
            if property.len() >= capacity { 
                return Err(format!("Table is at max capacity, cannot insert data into stream '{key}'"));
            }

            property.push(data);
            return Ok(());
        }

        return Err(format!("Expected stream with key '{key}', but none was found."));
    }

    /// swap remove an element's properties in the table at the specified index
    pub fn swap_remove(&mut self, idx: usize) {
        for prop in self.properties.values_mut() {
            prop.swap_remove(idx);
        }
    }

    /// Clear the property vectors in this data table. This does not clear the properties themselves.
    pub fn reset_properties(&mut self) {
        for dyn_vec in self.properties.values_mut() {
            dyn_vec.clear();
        }
    }

    /// Clear the DataTable of all properties. This preserves the capacity.
    pub fn clear(&mut self) {
        self.properties.clear();
    }
}