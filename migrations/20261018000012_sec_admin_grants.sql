-- Grant DELETE on security admin tables to the application role.
--
-- Permission-group management (/permissions/groups) replaces group
-- memberships and policy rules on update, and removes the group row on
-- delete. INSERT/UPDATE/SELECT were already granted in the baseline
-- security migration; DELETE is required for replace/remove semantics.

GRANT DELETE ON sec.principal_group_membership TO enginer_app;
GRANT DELETE ON sec.policy_rule TO enginer_app;
GRANT DELETE ON sec."group" TO enginer_app;
