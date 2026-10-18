-- Allow the app role to delete schema draft rows (`DELETE /schemas/drafts/<id>`).
GRANT DELETE ON meta.schema_revision TO enginer_app;
