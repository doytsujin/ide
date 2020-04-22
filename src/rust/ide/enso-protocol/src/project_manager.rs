//! Client library for the JSON-RPC-based Project Manager service.

#![warn(missing_docs)]
#![warn(trivial_casts)]
#![warn(trivial_numeric_casts)]
#![warn(unused_import_braces)]
#![warn(unused_qualifications)]
#![warn(unsafe_code)]
#![warn(missing_copy_implementations)]
#![warn(missing_debug_implementations)]

use enso_prelude::*;

use crate::common::UTCDateTime;
use crate::make_rpc_method;
use crate::make_rpc_methods;
use crate::make_param_map;
use crate::make_arg;
use json_rpc::api::Result;
use json_rpc::Handler;
use futures::Stream;
use serde::Serialize;
use serde::Deserialize;
use shapely::shared;
use std::future::Future;
use uuid::Uuid;



// =============
// === Event ===
// =============

/// Event emitted by the Project Manager `Client`.
pub type Event = json_rpc::handler::Event<Notification>;



// ====================
// === Notification ===
// ====================

/// Notification generated by the Project Manager.
#[derive(Clone,Copy,Debug,PartialEq)]
#[derive(Serialize, Deserialize)]
#[serde(tag="method", content="params")]
pub enum Notification {}



// ===================
// === RPC Methods ===
// ===================

// TODO[DG]: Wrap macro_rule with #[derive(JsonRpcInterface)]
make_rpc_methods! {
/// An interface containing all the available project management operations.
impl {
    /// Requests that the project picker open a specified project. This operation also
    /// includes spawning an instance of the language server open on the specified project.
    #[CamelCase=OpenProject,camelCase=openProject]
    fn open_project(&self, project_id:Uuid) -> IpWithSocket;

    /// Requests that the project picker close a specified project. This operation
    /// includes shutting down the language server gracefully so that it can persist state to disk as needed.
    #[CamelCase=CloseProject,camelCase=closeProject]
    fn close_project(&self, project_id:Uuid) -> ();

    /// Requests that the project picker lists the user's most recently opened
    /// projects.
    #[CamelCase=ListRecent,camelCase=listRecent]
    fn list_recent(&self, number_of_projects:u32) -> Vec<ProjectMetaData>;

    /// Requests the creation of a new project.
    #[CamelCase=CreateProject,camelCase=createProject]
    fn create_project(&self, name:String) -> Uuid;

    /// Requests the deletion of a project.
    #[CamelCase=DeleteProject,camelCase=deleteProject]
    fn delete_project(&self, project_id:Uuid) -> ();

    /// Requests a list of sample projects that are available to the user.
    #[CamelCase=ListSample,camelCase=listSample]
    fn list_sample(&self, number_of_projects:u32) -> Vec<ProjectMetaData>;
}
}

/// IP address with host and port.
#[derive(Debug,Clone,Serialize,Deserialize,PartialEq)]
pub struct IpWithSocket {
    host : String,
    port : u16
}

/// This type represents information about a project.
#[derive(Debug,Clone,Serialize,Deserialize,PartialEq)]
pub struct ProjectMetaData {
    name        : String,
    id          : Uuid,
    last_opened : UTCDateTime
}



// ============
// === Test ===
// ============

#[cfg(test)]
mod test {
    use super::Mock;
    use super::IpWithSocket;
    use super::ProjectMetaData;
    use super::Interface;
    use uuid::Uuid;
    use json_rpc::error::RpcError;
    use json_rpc::messages::Error;
    use json_rpc::Result;
    use std::future::Future;
    use utils::test::poll_future_output;

    fn error<T>(message:&str) -> Result<T> {
        let err = Error {
            code : 1,
            data : None,
            message : message.to_string()
        };
        Err(RpcError::RemoteError(err))
    }

    fn result<T,F:Future<Output = Result<T>>>(fut:F) -> Result<T> {
        let mut fut = Box::pin(fut);
        poll_future_output(&mut fut).expect("Promise isn't ready")
    }

    #[test]
    fn project_life_cycle() {
        let mock_client             = Mock::default();
        let expected_uuid           = Uuid::default();
        let host                    = "localhost".to_string();
        let port                    = 30500;
        let expected_ip_with_socket = IpWithSocket {host,port};
        mock_client.set_create_project_result("HelloWorld".into(),Ok(expected_uuid.clone()));
        mock_client.set_open_project_result(expected_uuid.clone(), Ok(expected_ip_with_socket.clone()));
        mock_client.set_close_project_result(expected_uuid.clone(), error("Project isn't open."));
        mock_client.set_delete_project_result(expected_uuid.clone(), error("Project doesn't exist."));

        let delete_result = mock_client.delete_project(expected_uuid.clone());
        result(delete_result).expect_err("Project shouldn't exist.");

        let uuid = mock_client.create_project("HelloWorld".into());
        let uuid = result(uuid).expect("Couldn't create project");
        assert_eq!(uuid, expected_uuid);

        let close_result = result(mock_client.close_project(uuid.clone()));
        close_result.expect_err("Project shouldn't be open.");

        let ip_with_socket = result(mock_client.open_project(uuid.clone()));
        let ip_with_socket = ip_with_socket.expect("Couldn't open project");
        assert_eq!(ip_with_socket, expected_ip_with_socket);

        mock_client.set_close_project_result(expected_uuid.clone(), Ok(()));
        result(mock_client.close_project(uuid)).expect("Couldn't close project.");

        mock_client.set_delete_project_result(expected_uuid.clone(), Ok(()));
        result(mock_client.delete_project(uuid)).expect("Couldn't delete project.");
    }

    #[test]
    fn list_projects() {
        let mock_client = Mock::default();
        let project1    = ProjectMetaData {
            name        : "project1".to_string(),
            id          : Uuid::default(),
            last_opened : chrono::DateTime::parse_from_rfc3339("2020-01-07T21:25:26Z").unwrap()
        };
        let project2 = ProjectMetaData {
            name        : "project2".to_string(),
            id          : Uuid::default(),
            last_opened : chrono::DateTime::parse_from_rfc3339("2020-02-02T13:15:20Z").unwrap()
        };
        let expected_recent_projects = vec![project1,project2];
        let sample1 = ProjectMetaData {
            name        : "sample1".to_string(),
            id          : Uuid::default(),
            last_opened : chrono::DateTime::parse_from_rfc3339("2019-11-23T05:30:12Z").unwrap()
        };
        let sample2 = ProjectMetaData {
            name        : "sample2".to_string(),
            id          : Uuid::default(),
            last_opened : chrono::DateTime::parse_from_rfc3339("2019-12-25T00:10:58Z").unwrap()
        };
        let expected_sample_projects = vec![sample1,sample2];
        mock_client.set_list_recent_result(2,Ok(expected_recent_projects.clone()));
        mock_client.set_list_sample_result(2,Ok(expected_sample_projects.clone()));

        let recent_projects = result(mock_client.list_recent(2)).expect("Couldn't get recent projects.");
        assert_eq!(recent_projects, expected_recent_projects);
        let sample_projects = result(mock_client.list_sample(2)).expect("Couldn't get sample projects.");
        assert_eq!(sample_projects, expected_sample_projects);
    }
}
