use graphql_client::GraphQLQuery;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "$schema_path$",
    query_path = "src/graphql/query/backend_query.gql",
    response_derives = "Debug"
)]
pub struct LogInAsVulcast;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "$schema_path$",
    query_path = "src/graphql/query/backend_query.gql",
    response_derives = "Debug"
)]
pub struct AssignVulcastToRelay;
