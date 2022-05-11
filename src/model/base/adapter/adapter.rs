// use rocket::futures::stream::SelectNextSome;
//
// pub struct tableField{
//     pub name:String,
//     pub kind:String,
//     pub default:Option<String>,
//     pub value:Option<String>,
//     pub isNull:bool,
//     pub index:bool
// }
//
// pub struct tableSql{
//     pub fields:Vec<tableField>,
//     pub kind:String,
//     pub name:String
// }
//
// pub trait tSqlAdapter {
//     fn getCreateTableSql(&self)->String;
//     fn getAlterTableSql(&self,updateTable:&tableSql)->String;
//     fn getInsertSql(&self)->String;
//     fn getUpdateSql(&self)->String;
// }
//
// pub trait tBaseAdapter {
//     fn createObject();
// }