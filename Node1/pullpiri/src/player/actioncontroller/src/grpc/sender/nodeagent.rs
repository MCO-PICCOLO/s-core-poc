use common::nodeagent::fromactioncontroller::{
    connect_server, HandleWorkloadRequest, HandleWorkloadResponse,
};
use common::nodeagent::node_agent_connection_client::NodeAgentConnectionClient;
use tonic::{Request, Status};

pub async fn send_workload_handle_request(
    addr: &str,
    request: HandleWorkloadRequest,
) -> Result<HandleWorkloadResponse, Status> {
    let mut client = NodeAgentConnectionClient::connect(connect_server(&addr))
        .await
        .unwrap();

    let response = client
        .handle_workload(Request::new(request))
        .await?
        .into_inner();
    Ok(response)
}
