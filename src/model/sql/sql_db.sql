create or replace function "public"."f_rise_enginer_altercolumn_helper" (tabns text, tabname text, colname text, newcoltype text, atttypmod int) returns bool as $$
declare
  v_attnum int;
  v_atttypmod int;
  v_typname text;
  v_query text;
  v_newcolname text;
  v_newcoltype text;
begin
  select a.attnum, a.atttypmod, d.typname into v_attnum, v_atttypmod, v_typname from pg_catalog.pg_type t join pg_catalog.pg_namespace n on (n.oid = t.typnamespace) join pg_catalog.pg_attribute a on (a.attstattarget = -1 and t.typrelid = a.attrelid) join pg_catalog.pg_type d on (d.oid=a.atttypid) where n.nspname=tabns and t.typname=tabname and a.attname=colname;
  v_newcolname := 'd' || v_attnum || colname;
  if newcoltype = 'varchar' then v_newcoltype := 'varchar(' || (atttypmod-4) || ')'; else v_newcoltype := newcoltype; end if;
  if v_typname <> newcoltype or v_atttypmod > atttypmod then
    execute 'alter table "' || tabns || '"."' || tabname || '" rename column "' || colname || '" to "' || v_newcolname || '"';
    execute 'alter table "' || tabns || '"."' || tabname || '" add column "' || colname || '" ' || v_newcoltype;
    execute 'update "' || tabns || '"."' || tabname || '" set "' || colname || '" = cast("' || v_newcolname || '" as ' || v_newcoltype || ')';
  elsif v_atttypmod < atttypmod then
    execute 'alter table "' || tabns || '"."' || tabname || '" alter column "' || colname || '" type ' || v_newcoltype;
  end if;
  return '1';
  exception
    when data_exception then return '0';
end;
$$ language plpgsql;
create or replace function "public"."f_rise_enginer"() returns integer as $$
declare
  v_model_id int;
begin
  -- Setup RISE
  if not exists(select * from pg_catalog.pg_tables where schemaname='public' and tablename='t_rise_u_model') then
    create table "public"."t_rise_u_model"
    (
      "c_id" bigserial not null,
      constraint "pk_rise_u_model_id" primary key ("c_id"),
      "c_u_name" varchar(50),
      "c_u_prefix" varchar(10),
      "c_u_guid" varchar(50),
      "c_u_version" int not null,
      "c_u_versionSequenceNumber" int not null
    );
    alter table "public"."t_rise_u_model" add constraint "ix_rise_u_model_u_guid unique" unique ("c_u_guid");
    alter table "public"."t_rise_u_model" add constraint "ix_rise_u_model_u_prefix" unique ("c_u_prefix");
    alter table "public"."t_rise_u_model" add constraint "ix_rise_u_model_u_name" unique ("c_u_name");
  end if;
  if not exists(select * from pg_catalog.pg_tables where schemaname='public' and tablename='t_rise_u_log') then
    create table "public"."t_rise_u_log"
    (
      "c_id" bigserial not null,
      constraint "pk_rise_u_log_id" primary key ("c_id"),
      "c_r_model" bigint not null,
      "c_u_sequenceNumber" int not null,
      "c_u_timeStamp" timestamp not null,
      "c_u_xml" text null
    );
    alter table "public"."t_rise_u_log" add constraint "fk_rise_u_log_r_model" foreign key ( "c_r_model" ) references  "public"."t_rise_u_model" ( "c_id" );
    alter table "public"."t_rise_u_log" add constraint "ix_rise_u_log_i_modelSN" unique ("c_r_model","c_u_sequenceNumber");
  end if;
  if not exists(select * from pg_catalog.pg_tables where schemaname='public' and tablename='t_rise_u_object') then
    create table "public"."t_rise_u_object"
    (
      "c_id" bigserial not null,
      constraint "pk_rise_u_object_id" primary key ("c_id"),
      "c_u_tableName" varchar(50) not null,
      "c_u_riseID" varchar(50) not null,
      "c_u_dbID" bigint not null,
      "c_u_state" varchar(1) not null
    );
    alter table "public"."t_rise_u_object" add constraint "ix_rise_u_object_u_tableNameriseID" unique ("c_u_tableName","c_u_riseID");
    alter table "public"."t_rise_u_object" add constraint "ix_rise_u_object_u_tableNamedbID" unique ("c_u_tableName","c_u_dbID");
  end if;
  if not exists (select * from pg_catalog.pg_type t join pg_catalog.pg_namespace n on (n.oid = t.typnamespace) join pg_catalog.pg_attribute a on (a.attstattarget = -1 and t.typrelid = a.attrelid) where n.nspname='public' and t.typname='t_rise_u_object' and  a.attname='c_u_state') then
    alter table "public"."t_rise_u_object" add column "c_u_state" varchar(1) null;
  end if;

  -- Install RISE model
  if not exists(select * from "public"."t_rise_u_model" where "c_u_prefix"='enginer' and "c_u_guid"='bd74e3ae-bca3-41f8-a0be-9cf5d8f9e943') then
    insert into "public"."t_rise_u_model" ("c_u_name","c_u_prefix","c_u_guid","c_u_version","c_u_versionSequenceNumber") values ('New Model','enginer','bd74e3ae-bca3-41f8-a0be-9cf5d8f9e943',0,0);
  end if;
  select into v_model_id c_id from "public"."t_rise_u_model" where "c_u_prefix"='enginer' and "c_u_guid"='bd74e3ae-bca3-41f8-a0be-9cf5d8f9e943';

  -- Sequence for naming primary key constraints
  if not exists(select * from pg_statio_all_sequences where schemaname='public' and relname='pk_enginer_constraint_seq') then
    create sequence "public"."pk_enginer_constraint_seq" start 1;
  end if;

  -- #1 Release point - Created from empty model Blank (,49944b71-12fc-41f2-81a9-bcde1d55782e)
  if not exists(select * from "public"."t_rise_u_log" where "c_r_model"=v_model_id and "c_u_sequenceNumber"=1) then
    insert into "public"."t_rise_u_log" ("c_r_model","c_u_sequenceNumber","c_u_timeStamp","c_u_xml") values (v_model_id,1,now(),'<rise:release xmlns:rise="http://www.r2bsoftware/ns/rise/"><rise:sequenceNumber>1</rise:sequenceNumber><rise:timeStamp>2022-05-15T04:04:22</rise:timeStamp><rise:comment>Created from empty model Blank (,49944b71-12fc-41f2-81a9-bcde1d55782e) </rise:comment></rise:release>');
    update "public"."t_rise_u_model" set "c_u_version"=1,"c_u_versionSequenceNumber"=1 where "c_u_prefix"='enginer' and "c_u_guid"='bd74e3ae-bca3-41f8-a0be-9cf5d8f9e943';
  end if;

  -- #2 Release point - Created from empty model Blank (,49944b71-12fc-41f2-81a9-bcde1d55782e)
  if not exists(select * from "public"."t_rise_u_log" where "c_r_model"=v_model_id and "c_u_sequenceNumber"=2) then
    insert into "public"."t_rise_u_log" ("c_r_model","c_u_sequenceNumber","c_u_timeStamp","c_u_xml") values (v_model_id,2,now(),'<rise:release xmlns:rise="http://www.r2bsoftware/ns/rise/"><rise:sequenceNumber>2</rise:sequenceNumber><rise:timeStamp>2022-05-15T04:05:01</rise:timeStamp><rise:comment>Created from empty model Blank (,49944b71-12fc-41f2-81a9-bcde1d55782e) </rise:comment></rise:release>');
    update "public"."t_rise_u_model" set "c_u_version"=2,"c_u_versionSequenceNumber"=2 where "c_u_prefix"='enginer' and "c_u_guid"='bd74e3ae-bca3-41f8-a0be-9cf5d8f9e943';
  end if;

  -- #3 New entity user with login, password, date_registred, is_active, date_last_active
  if not exists(select * from "public"."t_rise_u_log" where "c_r_model"=v_model_id and "c_u_sequenceNumber"=3) then
    insert into "public"."t_rise_u_log" ("c_r_model","c_u_sequenceNumber","c_u_timeStamp","c_u_xml") values (v_model_id,3,now(),'<rise:newEntity xmlns:rise="http://www.r2bsoftware/ns/rise/"><rise:sequenceNumber>3</rise:sequenceNumber><rise:timeStamp>2022-05-13T21:39:41</rise:timeStamp><rise:entity><rise:name>user</rise:name><rise:attribute><rise:name>login</rise:name><rise:dataTypeAlias /><rise:dataType>string</rise:dataType><rise:dataSize>255</rise:dataSize><rise:mustBeUnique>False</rise:mustBeUnique><rise:mustExist>True</rise:mustExist><rise:description>Login</rise:description></rise:attribute><rise:attribute><rise:name>password</rise:name><rise:dataTypeAlias /><rise:dataType>string</rise:dataType><rise:dataSize>256</rise:dataSize><rise:mustBeUnique>False</rise:mustBeUnique><rise:mustExist>True</rise:mustExist><rise:description>Password</rise:description></rise:attribute><rise:attribute><rise:name>date_registred</rise:name><rise:dataTypeAlias /><rise:dataType>datetime</rise:dataType><rise:mustBeUnique>False</rise:mustBeUnique><rise:mustExist>True</rise:mustExist></rise:attribute><rise:attribute><rise:name>is_active</rise:name><rise:dataTypeAlias /><rise:dataType>bool</rise:dataType><rise:mustBeUnique>False</rise:mustBeUnique><rise:mustExist>True</rise:mustExist><rise:default>0</rise:default></rise:attribute><rise:attribute><rise:name>date_last_active</rise:name><rise:dataTypeAlias /><rise:dataType>datetime</rise:dataType><rise:mustBeUnique>False</rise:mustBeUnique><rise:mustExist>False</rise:mustExist><rise:description /></rise:attribute><rise:maxID>0</rise:maxID></rise:entity></rise:newEntity>');
    if not exists (select * from pg_catalog.pg_tables where schemaname='public' and tablename='user') then
      create table  "public"."user"
      (
        "id" bigserial not null,
        "login" varchar(255) not null,
        "password" varchar(256) not null,
        "date_registred" timestamp not null,
        "is_active" bool not null,
        "date_last_active" timestamp null
      );
      execute 'alter table "public"."user" add constraint "pk_enginer_' || nextval('"public"."pk_enginer_constraint_seq"'::regclass) || '" primary key ("id");';
    end if;
    alter table "public"."user" alter column "is_active" set default 0;
  end if;

  -- #4 New entity group with alias, name, level
  if not exists(select * from "public"."t_rise_u_log" where "c_r_model"=v_model_id and "c_u_sequenceNumber"=4) then
    insert into "public"."t_rise_u_log" ("c_r_model","c_u_sequenceNumber","c_u_timeStamp","c_u_xml") values (v_model_id,4,now(),'<rise:newEntity xmlns:rise="http://www.r2bsoftware/ns/rise/"><rise:sequenceNumber>4</rise:sequenceNumber><rise:timeStamp>2022-05-13T21:39:41</rise:timeStamp><rise:entity><rise:name>group</rise:name><rise:attribute><rise:name>alias</rise:name><rise:dataTypeAlias /><rise:dataType>string</rise:dataType><rise:dataSize>255</rise:dataSize><rise:mustBeUnique>False</rise:mustBeUnique><rise:mustExist>True</rise:mustExist><rise:description /></rise:attribute><rise:attribute><rise:name>name</rise:name><rise:dataTypeAlias /><rise:dataType>string</rise:dataType><rise:dataSize>255</rise:dataSize><rise:mustBeUnique>False</rise:mustBeUnique><rise:mustExist>True</rise:mustExist><rise:description /></rise:attribute><rise:attribute><rise:name>level</rise:name><rise:dataTypeAlias /><rise:dataType>string</rise:dataType><rise:dataSize>50</rise:dataSize><rise:mustBeUnique>False</rise:mustBeUnique><rise:mustExist>True</rise:mustExist><rise:description /></rise:attribute><rise:maxID>0</rise:maxID></rise:entity></rise:newEntity>');
    if not exists (select * from pg_catalog.pg_tables where schemaname='public' and tablename='group') then
      create table  "public"."group"
      (
        "id" bigserial not null,
        "alias" varchar(255) not null,
        "name" varchar(255) not null,
        "level" varchar(50) not null
      );
      execute 'alter table "public"."group" add constraint "pk_enginer_' || nextval('"public"."pk_enginer_constraint_seq"'::regclass) || '" primary key ("id");';
    end if;
  end if;

  -- #5 New entity permission with alias, level, kind, object, access
  if not exists(select * from "public"."t_rise_u_log" where "c_r_model"=v_model_id and "c_u_sequenceNumber"=5) then
    insert into "public"."t_rise_u_log" ("c_r_model","c_u_sequenceNumber","c_u_timeStamp","c_u_xml") values (v_model_id,5,now(),'<rise:newEntity xmlns:rise="http://www.r2bsoftware/ns/rise/"><rise:sequenceNumber>5</rise:sequenceNumber><rise:timeStamp>2022-05-13T21:39:41</rise:timeStamp><rise:entity><rise:name>permission</rise:name><rise:attribute><rise:name>alias</rise:name><rise:dataTypeAlias /><rise:dataType>string</rise:dataType><rise:dataSize>50</rise:dataSize><rise:mustBeUnique>True</rise:mustBeUnique><rise:mustExist>True</rise:mustExist><rise:description /></rise:attribute><rise:attribute><rise:name>level</rise:name><rise:dataTypeAlias /><rise:dataType>string</rise:dataType><rise:dataSize>50</rise:dataSize><rise:mustBeUnique>False</rise:mustBeUnique><rise:mustExist>True</rise:mustExist><rise:description /></rise:attribute><rise:attribute><rise:name>kind</rise:name><rise:dataTypeAlias /><rise:dataType>string</rise:dataType><rise:dataSize>50</rise:dataSize><rise:mustBeUnique>False</rise:mustBeUnique><rise:mustExist>True</rise:mustExist><rise:description /></rise:attribute><rise:attribute><rise:name>object</rise:name><rise:dataTypeAlias /><rise:dataType>string</rise:dataType><rise:dataSize>50</rise:dataSize><rise:mustBeUnique>False</rise:mustBeUnique><rise:mustExist>True</rise:mustExist><rise:description /></rise:attribute><rise:attribute><rise:name>access</rise:name><rise:dataTypeAlias /><rise:dataType>string</rise:dataType><rise:dataSize>50</rise:dataSize><rise:mustBeUnique>False</rise:mustBeUnique><rise:mustExist>True</rise:mustExist><rise:description /></rise:attribute><rise:maxID>0</rise:maxID></rise:entity></rise:newEntity>');
    if not exists (select * from pg_catalog.pg_tables where schemaname='public' and tablename='permission') then
      create table  "public"."permission"
      (
        "id" bigserial not null,
        "alias" varchar(50) not null,
        "level" varchar(50) not null,
        "kind" varchar(50) not null,
        "object" varchar(50) not null,
        "access" varchar(50) not null
      );
      execute 'alter table "public"."permission" add constraint "pk_enginer_' || nextval('"public"."pk_enginer_constraint_seq"'::regclass) || '" primary key ("id");';
    end if;
    if not exists (select * from pg_catalog.pg_type t join pg_catalog.pg_namespace n on (n.oid = t.typnamespace) join pg_catalog.pg_constraint c on (c.conrelid=t.typrelid) where n.nspname='public' and c.conname='ix_enginer_permission_alias' and c.contype='u') then
      alter table "public"."permission" add constraint "ix_enginer_permission_alias" unique ("alias");
    end if;
  end if;

  -- #6 New entity object_type with alias, kind
  if not exists(select * from "public"."t_rise_u_log" where "c_r_model"=v_model_id and "c_u_sequenceNumber"=6) then
    insert into "public"."t_rise_u_log" ("c_r_model","c_u_sequenceNumber","c_u_timeStamp","c_u_xml") values (v_model_id,6,now(),'<rise:newEntity xmlns:rise="http://www.r2bsoftware/ns/rise/"><rise:sequenceNumber>6</rise:sequenceNumber><rise:timeStamp>2022-05-13T21:39:41</rise:timeStamp><rise:entity><rise:name>object_type</rise:name><rise:attribute><rise:name>alias</rise:name><rise:dataTypeAlias /><rise:dataType>string</rise:dataType><rise:dataSize>50</rise:dataSize><rise:mustBeUnique>True</rise:mustBeUnique><rise:mustExist>True</rise:mustExist><rise:description /></rise:attribute><rise:attribute><rise:name>kind</rise:name><rise:dataTypeAlias /><rise:dataType>string</rise:dataType><rise:dataSize>50</rise:dataSize><rise:mustBeUnique>False</rise:mustBeUnique><rise:mustExist>True</rise:mustExist><rise:description /></rise:attribute><rise:maxID>0</rise:maxID></rise:entity></rise:newEntity>');
    if not exists (select * from pg_catalog.pg_tables where schemaname='public' and tablename='object_type') then
      create table  "public"."object_type"
      (
        "id" bigserial not null,
        "alias" varchar(50) not null,
        "kind" varchar(50) not null
      );
      execute 'alter table "public"."object_type" add constraint "pk_enginer_' || nextval('"public"."pk_enginer_constraint_seq"'::regclass) || '" primary key ("id");';
    end if;
    if not exists (select * from pg_catalog.pg_type t join pg_catalog.pg_namespace n on (n.oid = t.typnamespace) join pg_catalog.pg_constraint c on (c.conrelid=t.typrelid) where n.nspname='public' and c.conname='ix_enginer_object_type_alias' and c.contype='u') then
      alter table "public"."object_type" add constraint "ix_enginer_object_type_alias" unique ("alias");
    end if;
  end if;

  -- #7 New entity link_type with alias, name
  if not exists(select * from "public"."t_rise_u_log" where "c_r_model"=v_model_id and "c_u_sequenceNumber"=7) then
    insert into "public"."t_rise_u_log" ("c_r_model","c_u_sequenceNumber","c_u_timeStamp","c_u_xml") values (v_model_id,7,now(),'<rise:newEntity xmlns:rise="http://www.r2bsoftware/ns/rise/"><rise:sequenceNumber>7</rise:sequenceNumber><rise:timeStamp>2022-05-13T21:39:41</rise:timeStamp><rise:entity><rise:name>link_type</rise:name><rise:attribute><rise:name>alias</rise:name><rise:dataTypeAlias /><rise:dataType>string</rise:dataType><rise:dataSize>50</rise:dataSize><rise:mustBeUnique>True</rise:mustBeUnique><rise:mustExist>True</rise:mustExist><rise:description /></rise:attribute><rise:attribute><rise:name>name</rise:name><rise:dataTypeAlias /><rise:dataType>string</rise:dataType><rise:dataSize>50</rise:dataSize><rise:mustBeUnique>False</rise:mustBeUnique><rise:mustExist>True</rise:mustExist><rise:description /></rise:attribute><rise:maxID>0</rise:maxID></rise:entity></rise:newEntity>');
    if not exists (select * from pg_catalog.pg_tables where schemaname='public' and tablename='link_type') then
      create table  "public"."link_type"
      (
        "id" bigserial not null,
        "alias" varchar(50) not null,
        "name" varchar(50) not null
      );
      execute 'alter table "public"."link_type" add constraint "pk_enginer_' || nextval('"public"."pk_enginer_constraint_seq"'::regclass) || '" primary key ("id");';
    end if;
    if not exists (select * from pg_catalog.pg_type t join pg_catalog.pg_namespace n on (n.oid = t.typnamespace) join pg_catalog.pg_constraint c on (c.conrelid=t.typrelid) where n.nspname='public' and c.conname='ix_enginer_link_type_alias' and c.contype='u') then
      alter table "public"."link_type" add constraint "ix_enginer_link_type_alias" unique ("alias");
    end if;
  end if;

  -- #8 New entity field with alias, name, kind, default, require, index, preview
  if not exists(select * from "public"."t_rise_u_log" where "c_r_model"=v_model_id and "c_u_sequenceNumber"=8) then
    insert into "public"."t_rise_u_log" ("c_r_model","c_u_sequenceNumber","c_u_timeStamp","c_u_xml") values (v_model_id,8,now(),'<rise:newEntity xmlns:rise="http://www.r2bsoftware/ns/rise/"><rise:sequenceNumber>8</rise:sequenceNumber><rise:timeStamp>2022-05-13T21:39:41</rise:timeStamp><rise:entity><rise:name>field</rise:name><rise:attribute><rise:name>alias</rise:name><rise:dataTypeAlias /><rise:dataType>string</rise:dataType><rise:dataSize>50</rise:dataSize><rise:mustBeUnique>True</rise:mustBeUnique><rise:mustExist>True</rise:mustExist><rise:description /></rise:attribute><rise:attribute><rise:name>name</rise:name><rise:dataTypeAlias /><rise:dataType>string</rise:dataType><rise:dataSize>50</rise:dataSize><rise:mustBeUnique>False</rise:mustBeUnique><rise:mustExist>True</rise:mustExist><rise:description /></rise:attribute><rise:attribute><rise:name>kind</rise:name><rise:dataTypeAlias /><rise:dataType>string</rise:dataType><rise:dataSize>50</rise:dataSize><rise:mustBeUnique>False</rise:mustBeUnique><rise:mustExist>True</rise:mustExist><rise:description /></rise:attribute><rise:attribute><rise:name>default</rise:name><rise:dataTypeAlias /><rise:dataType>string</rise:dataType><rise:dataSize>255</rise:dataSize><rise:mustBeUnique>False</rise:mustBeUnique><rise:mustExist>False</rise:mustExist><rise:description /></rise:attribute><rise:attribute><rise:name>require</rise:name><rise:dataTypeAlias /><rise:dataType>bool</rise:dataType><rise:mustBeUnique>False</rise:mustBeUnique><rise:mustExist>True</rise:mustExist><rise:default>0</rise:default><rise:description /></rise:attribute><rise:attribute><rise:name>index</rise:name><rise:dataTypeAlias /><rise:dataType>bool</rise:dataType><rise:mustBeUnique>False</rise:mustBeUnique><rise:mustExist>True</rise:mustExist><rise:default>0</rise:default><rise:description /></rise:attribute><rise:attribute><rise:name>preview</rise:name><rise:dataTypeAlias /><rise:dataType>bool</rise:dataType><rise:mustBeUnique>False</rise:mustBeUnique><rise:mustExist>True</rise:mustExist><rise:default>0</rise:default><rise:description /></rise:attribute><rise:maxID>0</rise:maxID></rise:entity></rise:newEntity>');
    if not exists (select * from pg_catalog.pg_tables where schemaname='public' and tablename='field') then
      create table  "public"."field"
      (
        "id" bigserial not null,
        "alias" varchar(50) not null,
        "name" varchar(50) not null,
        "kind" varchar(50) not null,
        "default" varchar(255) null,
        "require" bool not null,
        "index" bool not null,
        "preview" bool not null
      );
      execute 'alter table "public"."field" add constraint "pk_enginer_' || nextval('"public"."pk_enginer_constraint_seq"'::regclass) || '" primary key ("id");';
    end if;
    if not exists (select * from pg_catalog.pg_type t join pg_catalog.pg_namespace n on (n.oid = t.typnamespace) join pg_catalog.pg_constraint c on (c.conrelid=t.typrelid) where n.nspname='public' and c.conname='ix_enginer_field_alias' and c.contype='u') then
      alter table "public"."field" add constraint "ix_enginer_field_alias" unique ("alias");
    end if;
    alter table "public"."field" alter column "require" set default 0;
    alter table "public"."field" alter column "index" set default 0;
    alter table "public"."field" alter column "preview" set default 0;
  end if;

  -- #9 New entity object with aggr_object_type_alias, date_created, date_deleted
  if not exists(select * from "public"."t_rise_u_log" where "c_r_model"=v_model_id and "c_u_sequenceNumber"=9) then
    insert into "public"."t_rise_u_log" ("c_r_model","c_u_sequenceNumber","c_u_timeStamp","c_u_xml") values (v_model_id,9,now(),'<rise:newEntity xmlns:rise="http://www.r2bsoftware/ns/rise/"><rise:sequenceNumber>9</rise:sequenceNumber><rise:timeStamp>2022-05-13T21:39:41</rise:timeStamp><rise:entity><rise:name>object</rise:name><rise:attribute><rise:name>aggr_object_type_alias</rise:name><rise:dataTypeAlias /><rise:dataType>string</rise:dataType><rise:dataSize>50</rise:dataSize><rise:mustBeUnique>False</rise:mustBeUnique><rise:mustExist>True</rise:mustExist><rise:description /></rise:attribute><rise:attribute><rise:name>date_created</rise:name><rise:dataTypeAlias /><rise:dataType>datetime</rise:dataType><rise:mustBeUnique>False</rise:mustBeUnique><rise:mustExist>True</rise:mustExist><rise:description /></rise:attribute><rise:attribute><rise:name>date_deleted</rise:name><rise:dataTypeAlias /><rise:dataType>string</rise:dataType><rise:dataSize>50</rise:dataSize><rise:mustBeUnique>False</rise:mustBeUnique><rise:mustExist>False</rise:mustExist><rise:description /></rise:attribute><rise:maxID>0</rise:maxID></rise:entity></rise:newEntity>');
    if not exists (select * from pg_catalog.pg_tables where schemaname='public' and tablename='object') then
      create table  "public"."object"
      (
        "id" bigserial not null,
        "aggr_object_type_alias" varchar(50) not null,
        "date_created" timestamp not null,
        "date_deleted" varchar(50) null
      );
      execute 'alter table "public"."object" add constraint "pk_enginer_' || nextval('"public"."pk_enginer_constraint_seq"'::regclass) || '" primary key ("id");';
    end if;
  end if;

  -- #10 New entity link with date_created, date_deleted
  if not exists(select * from "public"."t_rise_u_log" where "c_r_model"=v_model_id and "c_u_sequenceNumber"=10) then
    insert into "public"."t_rise_u_log" ("c_r_model","c_u_sequenceNumber","c_u_timeStamp","c_u_xml") values (v_model_id,10,now(),'<rise:newEntity xmlns:rise="http://www.r2bsoftware/ns/rise/"><rise:sequenceNumber>10</rise:sequenceNumber><rise:timeStamp>2022-05-13T21:39:41</rise:timeStamp><rise:entity><rise:name>link</rise:name><rise:attribute><rise:name>date_created</rise:name><rise:dataTypeAlias /><rise:dataType>datetime</rise:dataType><rise:mustBeUnique>False</rise:mustBeUnique><rise:mustExist>True</rise:mustExist><rise:description /></rise:attribute><rise:attribute><rise:name>date_deleted</rise:name><rise:dataTypeAlias /><rise:dataType>datetime</rise:dataType><rise:mustBeUnique>False</rise:mustBeUnique><rise:mustExist>False</rise:mustExist><rise:description /></rise:attribute><rise:maxID>0</rise:maxID></rise:entity></rise:newEntity>');
    if not exists (select * from pg_catalog.pg_tables where schemaname='public' and tablename='link') then
      create table  "public"."link"
      (
        "id" bigserial not null,
        "date_created" timestamp not null,
        "date_deleted" timestamp null
      );
      execute 'alter table "public"."link" add constraint "pk_enginer_' || nextval('"public"."pk_enginer_constraint_seq"'::regclass) || '" primary key ("id");';
    end if;
  end if;

  -- #11 New relation user_group
  if not exists(select * from "public"."t_rise_u_log" where "c_r_model"=v_model_id and "c_u_sequenceNumber"=11) then
    insert into "public"."t_rise_u_log" ("c_r_model","c_u_sequenceNumber","c_u_timeStamp","c_u_xml") values (v_model_id,11,now(),'<rise:newRelation xmlns:rise="http://www.r2bsoftware/ns/rise/"><rise:sequenceNumber>11</rise:sequenceNumber><rise:timeStamp>2022-05-13T21:39:41</rise:timeStamp><rise:relation><rise:name>user_group</rise:name><rise:node><rise:name>user</rise:name><rise:entityName>user</rise:entityName><rise:cardinality>0toN</rise:cardinality></rise:node><rise:node><rise:name>group</rise:name><rise:entityName>group</rise:entityName><rise:cardinality>0toN</rise:cardinality></rise:node><rise:maxID>0</rise:maxID></rise:relation></rise:newRelation>');
    if not exists (select * from pg_catalog.pg_tables where schemaname='public' and tablename='r_user_group') then
      create table  "public"."r_user_group"
      (
        "id" bigserial not null,
        "user_id" bigint not null,
        "group_id" bigint not null
      );
      execute 'alter table "public"."r_user_group" add constraint "pk_enginer_' || nextval('"public"."pk_enginer_constraint_seq"'::regclass) || '" primary key ("id");';
    end if;
    if not exists (select * from pg_catalog.pg_type t join pg_catalog.pg_namespace n on (n.oid = t.typnamespace) join pg_catalog.pg_constraint c on (c.conrelid=t.typrelid) where n.nspname='public' and c.conname='fk_enginer_r_user_group_user_id' and c.contype='f') then
      alter table "public"."r_user_group" add constraint "fk_enginer_r_user_group_user_id" foreign key ("user_id") references "public"."user" ("id");
    end if;
    if not exists (select * from pg_catalog.pg_type t join pg_catalog.pg_namespace n on (n.oid = t.typnamespace) join pg_catalog.pg_constraint c on (c.conrelid=t.typrelid) where n.nspname='public' and c.conname='fk_enginer_r_user_group_group_id' and c.contype='f') then
      alter table "public"."r_user_group" add constraint "fk_enginer_r_user_group_group_id" foreign key ("group_id") references "public"."group" ("id");
    end if;
  end if;

  -- #12 New relation object_type_object
  if not exists(select * from "public"."t_rise_u_log" where "c_r_model"=v_model_id and "c_u_sequenceNumber"=12) then
    insert into "public"."t_rise_u_log" ("c_r_model","c_u_sequenceNumber","c_u_timeStamp","c_u_xml") values (v_model_id,12,now(),'<rise:newRelation xmlns:rise="http://www.r2bsoftware/ns/rise/"><rise:sequenceNumber>12</rise:sequenceNumber><rise:timeStamp>2022-05-13T21:39:41</rise:timeStamp><rise:relation><rise:name>object_type_object</rise:name><rise:node><rise:name>object_type</rise:name><rise:entityName>object_type</rise:entityName><rise:cardinality>1</rise:cardinality></rise:node><rise:node><rise:name>object</rise:name><rise:entityName>object</rise:entityName><rise:cardinality>0toN</rise:cardinality></rise:node><rise:maxID>0</rise:maxID></rise:relation></rise:newRelation>');
    if not exists (select * from pg_catalog.pg_type t join pg_catalog.pg_namespace n on (n.oid = t.typnamespace) join pg_catalog.pg_attribute a on (a.attstattarget = -1 and t.typrelid = a.attrelid) where n.nspname='public' and t.typname='object' and a.attname='object_type_id') then
      alter table "public"."object" add column "object_type_id" bigint null;
      alter table "public"."object" alter column "object_type_id" set not null;
    end if;
    if not exists (select * from pg_catalog.pg_type t join pg_catalog.pg_namespace n on (n.oid = t.typnamespace) join pg_catalog.pg_constraint c on (c.conrelid=t.typrelid) where n.nspname='public' and c.conname='fk_enginer_object_object_type_id' and c.contype='f') then
      alter table "public"."object" add constraint "fk_enginer_object_object_type_id" foreign key ("object_type_id") references "public"."object_type" ("id");
    end if;
  end if;

  -- #13 New relation object_type_to
  if not exists(select * from "public"."t_rise_u_log" where "c_r_model"=v_model_id and "c_u_sequenceNumber"=13) then
    insert into "public"."t_rise_u_log" ("c_r_model","c_u_sequenceNumber","c_u_timeStamp","c_u_xml") values (v_model_id,13,now(),'<rise:newRelation xmlns:rise="http://www.r2bsoftware/ns/rise/"><rise:sequenceNumber>13</rise:sequenceNumber><rise:timeStamp>2022-05-13T21:39:41</rise:timeStamp><rise:relation><rise:name>object_type_to</rise:name><rise:node><rise:name>object_type_to</rise:name><rise:entityName>object_type</rise:entityName><rise:cardinality>1</rise:cardinality></rise:node><rise:node><rise:name>link_type</rise:name><rise:entityName>link_type</rise:entityName><rise:cardinality>1toN</rise:cardinality></rise:node><rise:maxID>0</rise:maxID></rise:relation></rise:newRelation>');
    if not exists (select * from pg_catalog.pg_type t join pg_catalog.pg_namespace n on (n.oid = t.typnamespace) join pg_catalog.pg_attribute a on (a.attstattarget = -1 and t.typrelid = a.attrelid) where n.nspname='public' and t.typname='link_type' and a.attname='object_type_to_id') then
      alter table "public"."link_type" add column "object_type_to_id" bigint null;
      alter table "public"."link_type" alter column "object_type_to_id" set not null;
    end if;
    if not exists (select * from pg_catalog.pg_type t join pg_catalog.pg_namespace n on (n.oid = t.typnamespace) join pg_catalog.pg_constraint c on (c.conrelid=t.typrelid) where n.nspname='public' and c.conname='fk_enginer_link_type_object_type_to_id' and c.contype='f') then
      alter table "public"."link_type" add constraint "fk_enginer_link_type_object_type_to_id" foreign key ("object_type_to_id") references "public"."object_type" ("id");
    end if;
  end if;

  -- #14 New relation object_type_from
  if not exists(select * from "public"."t_rise_u_log" where "c_r_model"=v_model_id and "c_u_sequenceNumber"=14) then
    insert into "public"."t_rise_u_log" ("c_r_model","c_u_sequenceNumber","c_u_timeStamp","c_u_xml") values (v_model_id,14,now(),'<rise:newRelation xmlns:rise="http://www.r2bsoftware/ns/rise/"><rise:sequenceNumber>14</rise:sequenceNumber><rise:timeStamp>2022-05-13T21:39:41</rise:timeStamp><rise:relation><rise:name>object_type_from</rise:name><rise:node><rise:name>object_type_from</rise:name><rise:entityName>object_type</rise:entityName><rise:cardinality>1</rise:cardinality></rise:node><rise:node><rise:name>object_type</rise:name><rise:entityName>link_type</rise:entityName><rise:cardinality>1toN</rise:cardinality></rise:node><rise:maxID>0</rise:maxID></rise:relation></rise:newRelation>');
    if not exists (select * from pg_catalog.pg_type t join pg_catalog.pg_namespace n on (n.oid = t.typnamespace) join pg_catalog.pg_attribute a on (a.attstattarget = -1 and t.typrelid = a.attrelid) where n.nspname='public' and t.typname='link_type' and a.attname='object_type_from_id') then
      alter table "public"."link_type" add column "object_type_from_id" bigint null;
      alter table "public"."link_type" alter column "object_type_from_id" set not null;
    end if;
    if not exists (select * from pg_catalog.pg_type t join pg_catalog.pg_namespace n on (n.oid = t.typnamespace) join pg_catalog.pg_constraint c on (c.conrelid=t.typrelid) where n.nspname='public' and c.conname='fk_enginer_link_type_object_type_from_id' and c.contype='f') then
      alter table "public"."link_type" add constraint "fk_enginer_link_type_object_type_from_id" foreign key ("object_type_from_id") references "public"."object_type" ("id");
    end if;
  end if;

  -- #15 New relation group_permission
  if not exists(select * from "public"."t_rise_u_log" where "c_r_model"=v_model_id and "c_u_sequenceNumber"=15) then
    insert into "public"."t_rise_u_log" ("c_r_model","c_u_sequenceNumber","c_u_timeStamp","c_u_xml") values (v_model_id,15,now(),'<rise:newRelation xmlns:rise="http://www.r2bsoftware/ns/rise/"><rise:sequenceNumber>15</rise:sequenceNumber><rise:timeStamp>2022-05-13T21:39:41</rise:timeStamp><rise:relation><rise:name>group_permission</rise:name><rise:node><rise:name>group</rise:name><rise:entityName>group</rise:entityName><rise:cardinality>0or1</rise:cardinality></rise:node><rise:node><rise:name>permission</rise:name><rise:entityName>permission</rise:entityName><rise:cardinality>0toN</rise:cardinality></rise:node><rise:maxID>0</rise:maxID></rise:relation></rise:newRelation>');
    if not exists (select * from pg_catalog.pg_type t join pg_catalog.pg_namespace n on (n.oid = t.typnamespace) join pg_catalog.pg_attribute a on (a.attstattarget = -1 and t.typrelid = a.attrelid) where n.nspname='public' and t.typname='permission' and a.attname='group_id') then
      alter table "public"."permission" add column "group_id" bigint null;
    end if;
    if not exists (select * from pg_catalog.pg_type t join pg_catalog.pg_namespace n on (n.oid = t.typnamespace) join pg_catalog.pg_constraint c on (c.conrelid=t.typrelid) where n.nspname='public' and c.conname='fk_enginer_permission_group_id' and c.contype='f') then
      alter table "public"."permission" add constraint "fk_enginer_permission_group_id" foreign key ("group_id") references "public"."group" ("id");
    end if;
  end if;

  -- #16 New relation user_created_object
  if not exists(select * from "public"."t_rise_u_log" where "c_r_model"=v_model_id and "c_u_sequenceNumber"=16) then
    insert into "public"."t_rise_u_log" ("c_r_model","c_u_sequenceNumber","c_u_timeStamp","c_u_xml") values (v_model_id,16,now(),'<rise:newRelation xmlns:rise="http://www.r2bsoftware/ns/rise/"><rise:sequenceNumber>16</rise:sequenceNumber><rise:timeStamp>2022-05-13T21:39:41</rise:timeStamp><rise:relation><rise:name>user_created_object</rise:name><rise:node><rise:name>user_created</rise:name><rise:entityName>user</rise:entityName><rise:cardinality>0or1</rise:cardinality></rise:node><rise:node><rise:name>id</rise:name><rise:entityName>object</rise:entityName><rise:cardinality>0toN</rise:cardinality></rise:node><rise:maxID>0</rise:maxID></rise:relation></rise:newRelation>');
    if not exists (select * from pg_catalog.pg_type t join pg_catalog.pg_namespace n on (n.oid = t.typnamespace) join pg_catalog.pg_attribute a on (a.attstattarget = -1 and t.typrelid = a.attrelid) where n.nspname='public' and t.typname='object' and a.attname='user_created_id') then
      alter table "public"."object" add column "user_created_id" bigint null;
    end if;
    if not exists (select * from pg_catalog.pg_type t join pg_catalog.pg_namespace n on (n.oid = t.typnamespace) join pg_catalog.pg_constraint c on (c.conrelid=t.typrelid) where n.nspname='public' and c.conname='fk_enginer_object_user_created_id' and c.contype='f') then
      alter table "public"."object" add constraint "fk_enginer_object_user_created_id" foreign key ("user_created_id") references "public"."user" ("id");
    end if;
  end if;

  -- #17 New relation user_deleted_object
  if not exists(select * from "public"."t_rise_u_log" where "c_r_model"=v_model_id and "c_u_sequenceNumber"=17) then
    insert into "public"."t_rise_u_log" ("c_r_model","c_u_sequenceNumber","c_u_timeStamp","c_u_xml") values (v_model_id,17,now(),'<rise:newRelation xmlns:rise="http://www.r2bsoftware/ns/rise/"><rise:sequenceNumber>17</rise:sequenceNumber><rise:timeStamp>2022-05-13T21:39:41</rise:timeStamp><rise:relation><rise:name>user_deleted_object</rise:name><rise:node><rise:name>user_deleted</rise:name><rise:entityName>user</rise:entityName><rise:cardinality>0or1</rise:cardinality></rise:node><rise:node><rise:name>object</rise:name><rise:entityName>object</rise:entityName><rise:cardinality>0toN</rise:cardinality></rise:node><rise:maxID>0</rise:maxID></rise:relation></rise:newRelation>');
    if not exists (select * from pg_catalog.pg_type t join pg_catalog.pg_namespace n on (n.oid = t.typnamespace) join pg_catalog.pg_attribute a on (a.attstattarget = -1 and t.typrelid = a.attrelid) where n.nspname='public' and t.typname='object' and a.attname='user_deleted_id') then
      alter table "public"."object" add column "user_deleted_id" bigint null;
    end if;
    if not exists (select * from pg_catalog.pg_type t join pg_catalog.pg_namespace n on (n.oid = t.typnamespace) join pg_catalog.pg_constraint c on (c.conrelid=t.typrelid) where n.nspname='public' and c.conname='fk_enginer_object_user_deleted_id' and c.contype='f') then
      alter table "public"."object" add constraint "fk_enginer_object_user_deleted_id" foreign key ("user_deleted_id") references "public"."user" ("id");
    end if;
  end if;

  -- #18 New relation object_type_field
  if not exists(select * from "public"."t_rise_u_log" where "c_r_model"=v_model_id and "c_u_sequenceNumber"=18) then
    insert into "public"."t_rise_u_log" ("c_r_model","c_u_sequenceNumber","c_u_timeStamp","c_u_xml") values (v_model_id,18,now(),'<rise:newRelation xmlns:rise="http://www.r2bsoftware/ns/rise/"><rise:sequenceNumber>18</rise:sequenceNumber><rise:timeStamp>2022-05-13T21:39:41</rise:timeStamp><rise:relation><rise:name>object_type_field</rise:name><rise:node><rise:name>object_type</rise:name><rise:entityName>object_type</rise:entityName><rise:cardinality>1</rise:cardinality></rise:node><rise:node><rise:name>field</rise:name><rise:entityName>field</rise:entityName><rise:cardinality>0toN</rise:cardinality></rise:node><rise:maxID>0</rise:maxID></rise:relation></rise:newRelation>');
    if not exists (select * from pg_catalog.pg_type t join pg_catalog.pg_namespace n on (n.oid = t.typnamespace) join pg_catalog.pg_attribute a on (a.attstattarget = -1 and t.typrelid = a.attrelid) where n.nspname='public' and t.typname='field' and a.attname='object_type_id') then
      alter table "public"."field" add column "object_type_id" bigint null;
      alter table "public"."field" alter column "object_type_id" set not null;
    end if;
    if not exists (select * from pg_catalog.pg_type t join pg_catalog.pg_namespace n on (n.oid = t.typnamespace) join pg_catalog.pg_constraint c on (c.conrelid=t.typrelid) where n.nspname='public' and c.conname='fk_enginer_field_object_type_id' and c.contype='f') then
      alter table "public"."field" add constraint "fk_enginer_field_object_type_id" foreign key ("object_type_id") references "public"."object_type" ("id");
    end if;
  end if;

  -- #19 New relation link_type_link
  if not exists(select * from "public"."t_rise_u_log" where "c_r_model"=v_model_id and "c_u_sequenceNumber"=19) then
    insert into "public"."t_rise_u_log" ("c_r_model","c_u_sequenceNumber","c_u_timeStamp","c_u_xml") values (v_model_id,19,now(),'<rise:newRelation xmlns:rise="http://www.r2bsoftware/ns/rise/"><rise:sequenceNumber>19</rise:sequenceNumber><rise:timeStamp>2022-05-13T21:39:41</rise:timeStamp><rise:relation><rise:name>link_type_link</rise:name><rise:node><rise:name>link_type</rise:name><rise:entityName>link_type</rise:entityName><rise:cardinality>1</rise:cardinality></rise:node><rise:node><rise:name>link</rise:name><rise:entityName>link</rise:entityName><rise:cardinality>0toN</rise:cardinality></rise:node><rise:maxID>0</rise:maxID></rise:relation></rise:newRelation>');
    if not exists (select * from pg_catalog.pg_type t join pg_catalog.pg_namespace n on (n.oid = t.typnamespace) join pg_catalog.pg_attribute a on (a.attstattarget = -1 and t.typrelid = a.attrelid) where n.nspname='public' and t.typname='link' and a.attname='link_type_id') then
      alter table "public"."link" add column "link_type_id" bigint null;
      alter table "public"."link" alter column "link_type_id" set not null;
    end if;
    if not exists (select * from pg_catalog.pg_type t join pg_catalog.pg_namespace n on (n.oid = t.typnamespace) join pg_catalog.pg_constraint c on (c.conrelid=t.typrelid) where n.nspname='public' and c.conname='fk_enginer_link_link_type_id' and c.contype='f') then
      alter table "public"."link" add constraint "fk_enginer_link_link_type_id" foreign key ("link_type_id") references "public"."link_type" ("id");
    end if;
  end if;

  -- #20 New relation object_to
  if not exists(select * from "public"."t_rise_u_log" where "c_r_model"=v_model_id and "c_u_sequenceNumber"=20) then
    insert into "public"."t_rise_u_log" ("c_r_model","c_u_sequenceNumber","c_u_timeStamp","c_u_xml") values (v_model_id,20,now(),'<rise:newRelation xmlns:rise="http://www.r2bsoftware/ns/rise/"><rise:sequenceNumber>20</rise:sequenceNumber><rise:timeStamp>2022-05-13T21:39:41</rise:timeStamp><rise:relation><rise:name>object_to</rise:name><rise:node><rise:name>object_to</rise:name><rise:entityName>object</rise:entityName><rise:cardinality>1</rise:cardinality></rise:node><rise:node><rise:name>link</rise:name><rise:entityName>link</rise:entityName><rise:cardinality>0toN</rise:cardinality></rise:node><rise:maxID>0</rise:maxID></rise:relation></rise:newRelation>');
    if not exists (select * from pg_catalog.pg_type t join pg_catalog.pg_namespace n on (n.oid = t.typnamespace) join pg_catalog.pg_attribute a on (a.attstattarget = -1 and t.typrelid = a.attrelid) where n.nspname='public' and t.typname='link' and a.attname='object_to_id') then
      alter table "public"."link" add column "object_to_id" bigint null;
      alter table "public"."link" alter column "object_to_id" set not null;
    end if;
    if not exists (select * from pg_catalog.pg_type t join pg_catalog.pg_namespace n on (n.oid = t.typnamespace) join pg_catalog.pg_constraint c on (c.conrelid=t.typrelid) where n.nspname='public' and c.conname='fk_enginer_link_object_to_id' and c.contype='f') then
      alter table "public"."link" add constraint "fk_enginer_link_object_to_id" foreign key ("object_to_id") references "public"."object" ("id");
    end if;
  end if;

  -- #21 New relation object_from
  if not exists(select * from "public"."t_rise_u_log" where "c_r_model"=v_model_id and "c_u_sequenceNumber"=21) then
    insert into "public"."t_rise_u_log" ("c_r_model","c_u_sequenceNumber","c_u_timeStamp","c_u_xml") values (v_model_id,21,now(),'<rise:newRelation xmlns:rise="http://www.r2bsoftware/ns/rise/"><rise:sequenceNumber>21</rise:sequenceNumber><rise:timeStamp>2022-05-13T21:39:41</rise:timeStamp><rise:relation><rise:name>object_from</rise:name><rise:node><rise:name>object_from</rise:name><rise:entityName>object</rise:entityName><rise:cardinality>1</rise:cardinality></rise:node><rise:node><rise:name>link1</rise:name><rise:entityName>link</rise:entityName><rise:cardinality>0toN</rise:cardinality></rise:node><rise:maxID>0</rise:maxID></rise:relation></rise:newRelation>');
    if not exists (select * from pg_catalog.pg_type t join pg_catalog.pg_namespace n on (n.oid = t.typnamespace) join pg_catalog.pg_attribute a on (a.attstattarget = -1 and t.typrelid = a.attrelid) where n.nspname='public' and t.typname='link' and a.attname='object_from_id') then
      alter table "public"."link" add column "object_from_id" bigint null;
      alter table "public"."link" alter column "object_from_id" set not null;
    end if;
    if not exists (select * from pg_catalog.pg_type t join pg_catalog.pg_namespace n on (n.oid = t.typnamespace) join pg_catalog.pg_constraint c on (c.conrelid=t.typrelid) where n.nspname='public' and c.conname='fk_enginer_link_object_from_id' and c.contype='f') then
      alter table "public"."link" add constraint "fk_enginer_link_object_from_id" foreign key ("object_from_id") references "public"."object" ("id");
    end if;
  end if;

  -- #22 New relation user_deleted_link
  if not exists(select * from "public"."t_rise_u_log" where "c_r_model"=v_model_id and "c_u_sequenceNumber"=22) then
    insert into "public"."t_rise_u_log" ("c_r_model","c_u_sequenceNumber","c_u_timeStamp","c_u_xml") values (v_model_id,22,now(),'<rise:newRelation xmlns:rise="http://www.r2bsoftware/ns/rise/"><rise:sequenceNumber>22</rise:sequenceNumber><rise:timeStamp>2022-05-13T21:39:41</rise:timeStamp><rise:relation><rise:name>user_deleted_link</rise:name><rise:node><rise:name>user_deleted</rise:name><rise:entityName>user</rise:entityName><rise:cardinality>1</rise:cardinality></rise:node><rise:node><rise:name>link</rise:name><rise:entityName>link</rise:entityName><rise:cardinality>0toN</rise:cardinality></rise:node><rise:maxID>0</rise:maxID></rise:relation></rise:newRelation>');
    if not exists (select * from pg_catalog.pg_type t join pg_catalog.pg_namespace n on (n.oid = t.typnamespace) join pg_catalog.pg_attribute a on (a.attstattarget = -1 and t.typrelid = a.attrelid) where n.nspname='public' and t.typname='link' and a.attname='user_deleted_id') then
      alter table "public"."link" add column "user_deleted_id" bigint null;
      alter table "public"."link" alter column "user_deleted_id" set not null;
    end if;
    if not exists (select * from pg_catalog.pg_type t join pg_catalog.pg_namespace n on (n.oid = t.typnamespace) join pg_catalog.pg_constraint c on (c.conrelid=t.typrelid) where n.nspname='public' and c.conname='fk_enginer_link_user_deleted_id' and c.contype='f') then
      alter table "public"."link" add constraint "fk_enginer_link_user_deleted_id" foreign key ("user_deleted_id") references "public"."user" ("id");
    end if;
  end if;

  -- #23 New relation user_created_link
  if not exists(select * from "public"."t_rise_u_log" where "c_r_model"=v_model_id and "c_u_sequenceNumber"=23) then
    insert into "public"."t_rise_u_log" ("c_r_model","c_u_sequenceNumber","c_u_timeStamp","c_u_xml") values (v_model_id,23,now(),'<rise:newRelation xmlns:rise="http://www.r2bsoftware/ns/rise/"><rise:sequenceNumber>23</rise:sequenceNumber><rise:timeStamp>2022-05-13T21:39:41</rise:timeStamp><rise:relation><rise:name>user_created_link</rise:name><rise:node><rise:name>user_created</rise:name><rise:entityName>user</rise:entityName><rise:cardinality>1</rise:cardinality></rise:node><rise:node><rise:name>link1</rise:name><rise:entityName>link</rise:entityName><rise:cardinality>0toN</rise:cardinality></rise:node><rise:maxID>0</rise:maxID></rise:relation></rise:newRelation>');
    if not exists (select * from pg_catalog.pg_type t join pg_catalog.pg_namespace n on (n.oid = t.typnamespace) join pg_catalog.pg_attribute a on (a.attstattarget = -1 and t.typrelid = a.attrelid) where n.nspname='public' and t.typname='link' and a.attname='user_created_id') then
      alter table "public"."link" add column "user_created_id" bigint null;
      alter table "public"."link" alter column "user_created_id" set not null;
    end if;
    if not exists (select * from pg_catalog.pg_type t join pg_catalog.pg_namespace n on (n.oid = t.typnamespace) join pg_catalog.pg_constraint c on (c.conrelid=t.typrelid) where n.nspname='public' and c.conname='fk_enginer_link_user_created_id' and c.contype='f') then
      alter table "public"."link" add constraint "fk_enginer_link_user_created_id" foreign key ("user_created_id") references "public"."user" ("id");
    end if;
  end if;

  -- #24 Release point - Created from snapshot of model enginer (enginer,a843c315-b92d-4516-9592-4fee4174b233) at evolution #22
  if not exists(select * from "public"."t_rise_u_log" where "c_r_model"=v_model_id and "c_u_sequenceNumber"=24) then
    insert into "public"."t_rise_u_log" ("c_r_model","c_u_sequenceNumber","c_u_timeStamp","c_u_xml") values (v_model_id,24,now(),'<rise:release xmlns:rise="http://www.r2bsoftware/ns/rise/"><rise:sequenceNumber>24</rise:sequenceNumber><rise:timeStamp>2022-05-15T04:05:01</rise:timeStamp><rise:comment>Created from snapshot of model enginer (enginer,a843c315-b92d-4516-9592-4fee4174b233) at evolution #22</rise:comment></rise:release>');
    update "public"."t_rise_u_model" set "c_u_version"=3,"c_u_versionSequenceNumber"=24 where "c_u_prefix"='enginer' and "c_u_guid"='bd74e3ae-bca3-41f8-a0be-9cf5d8f9e943';
  end if;

  -- #25 New permission attributes name
  if not exists(select * from "public"."t_rise_u_log" where "c_r_model"=v_model_id and "c_u_sequenceNumber"=25) then
    insert into "public"."t_rise_u_log" ("c_r_model","c_u_sequenceNumber","c_u_timeStamp","c_u_xml") values (v_model_id,25,now(),'<rise:newAttribute xmlns:rise="http://www.r2bsoftware/ns/rise/"><rise:sequenceNumber>25</rise:sequenceNumber><rise:timeStamp>2022-05-16T01:00:44</rise:timeStamp><rise:entity><rise:name>permission</rise:name><rise:attribute><rise:name>name</rise:name><rise:dataTypeAlias /><rise:dataType>string</rise:dataType><rise:dataSize>50</rise:dataSize><rise:mustBeUnique>False</rise:mustBeUnique><rise:mustExist>True</rise:mustExist><rise:description /></rise:attribute><rise:maxID>0</rise:maxID></rise:entity></rise:newAttribute>');
    if not exists (select * from pg_catalog.pg_type t join pg_catalog.pg_namespace n on (n.oid = t.typnamespace) join pg_catalog.pg_attribute a on (a.attstattarget = -1 and t.typrelid = a.attrelid) where n.nspname='public' and t.typname='permission' and a.attname='name') then
      alter table "public"."permission" add column "name" varchar(50) null;
      alter table "public"."permission" alter column "name" set not null;
    end if;
  end if;

  return 25;
end;
$$ language plpgsql;
select "public"."f_rise_enginer"();
drop function "public"."f_rise_enginer"();
drop function "public"."f_rise_enginer_altercolumn_helper"(tabns text, tabname text, colname text, newcoltype text, atttypmod int);

