use traits::Db;

#[dbstruct::dbstruct(db=sled)]
pub struct DerivedDb {

}

impl Db for DerivedDb {
    type Error = dbstruct::Error<dbstruct::sled::Error>;
    fn open(path: &std::path::Path) -> Result<Self, Self::Error> {
        let db = DerivedDb::new(path)?;
        Ok(db)
    }
}
