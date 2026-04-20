use std::collections::{HashMap, HashSet};

use cycms_content_model::ContentModelRegistry;
use cycms_core::Result;
use cycms_plugin_api::PluginRouteDoc;
use cycms_plugin_native::NativePluginRuntime;
use cycms_plugin_wasm::WasmPluginRuntime;
use serde_json::{Map, Value, json};

macro_rules! map {
	($($key:literal => $value:expr),* $(,)?) => {{
		let mut map = Map::new();
		$(
			map.insert($key.to_owned(), $value);
		)*
		map
	}};
}

pub async fn build_openapi_json(
	content_model: &ContentModelRegistry,
	native_runtime: &NativePluginRuntime,
	wasm_runtime: &WasmPluginRuntime,
) -> Result<Value> {
	let mut schemas = static_schemas();
	let mut paths = Map::<String, Value>::new();

	add_static_paths(&mut paths);

	for content_type in content_model.list_types().await? {
		let fields_schema_name = format!("ContentFields{}", pascal_case(&content_type.api_id));
		let entry_schema_name = format!("ContentEntry{}", pascal_case(&content_type.api_id));
		let fields_schema = content_model.to_json_schema(&content_type.api_id).await?;
		schemas.insert(fields_schema_name.clone(), fields_schema);
		schemas.insert(entry_schema_name.clone(), content_entry_schema(&fields_schema_name));
		add_dynamic_content_paths(&mut paths, &content_type.api_id, &entry_schema_name);
	}

	add_plugin_paths(
		&mut paths,
		native_runtime.all_route_docs(),
		native_runtime
			.all_routes()
			.into_iter()
			.map(|(name, _)| name)
			.collect(),
	);
	add_plugin_paths(
		&mut paths,
		wasm_runtime.all_route_docs(),
		wasm_runtime
			.all_routes()
			.into_iter()
			.map(|(name, _)| name)
			.collect(),
	);

	let document = json!({
		"openapi": "3.1.0",
		"info": {
			"title": "cycms API",
			"version": env!("CARGO_PKG_VERSION"),
			"description": "cycms v0.1 management and plugin API"
		},
		"tags": [
			{ "name": "auth" },
			{ "name": "content-types" },
			{ "name": "content" },
			{ "name": "media" },
			{ "name": "plugins" },
			{ "name": "settings" },
			{ "name": "users" },
			{ "name": "roles" }
		],
		"paths": Value::Object(paths),
		"components": {
			"securitySchemes": {
				"bearerAuth": {
					"type": "http",
					"scheme": "bearer",
					"bearerFormat": "JWT"
				}
			},
			"schemas": Value::Object(schemas)
		}
	});

	Ok(document)
}

fn add_static_paths(paths: &mut Map<String, Value>) {
	insert_operation(paths, "/api/docs", "get", operation("获取 OpenAPI 文档", "auth", false, None, None, map! {
		"200" => json_response(json!({ "type": "object", "additionalProperties": true }), "OpenAPI JSON")
	}));

	insert_operation(paths, "/api/v1/auth/login", "post", operation("用户登录", "auth", false, Some(json!({
		"required": true,
		"content": { "application/json": { "schema": {
			"type": "object",
			"required": ["username", "password"],
			"properties": {
				"username": { "type": "string" },
				"password": { "type": "string", "format": "password" }
			}
		} } }
	})), None, map! {
		"200" => json_response(schema_ref("TokenPairResponse"), "登录成功"),
		"401" => json_response(schema_ref("ErrorResponse"), "凭证错误")
	}));
	insert_operation(paths, "/api/v1/auth/register", "post", operation("创建初始管理员", "auth", false, Some(json!({
		"required": true,
		"content": { "application/json": { "schema": {
			"type": "object",
			"required": ["username", "email", "password"],
			"properties": {
				"username": { "type": "string" },
				"email": { "type": "string", "format": "email" },
				"password": { "type": "string", "format": "password" }
			}
		} } }
	})), None, map! {
		"201" => json_response(schema_ref("UserResponse"), "管理员创建成功"),
		"409" => json_response(schema_ref("ErrorResponse"), "系统已初始化")
	}));
	insert_operation(paths, "/api/v1/auth/refresh", "post", operation("刷新访问令牌", "auth", false, Some(json!({
		"required": true,
		"content": { "application/json": { "schema": {
			"type": "object",
			"required": ["refresh_token"],
			"properties": {
				"refresh_token": { "type": "string" }
			}
		} } }
	})), None, map! {
		"200" => json_response(schema_ref("TokenPairResponse"), "刷新成功"),
		"401" => json_response(schema_ref("ErrorResponse"), "refresh token 无效")
	}));
	insert_operation(paths, "/api/v1/auth/me", "get", operation("获取当前用户", "auth", true, None, None, map! {
		"200" => json_response(schema_ref("UserResponse"), "当前用户"),
		"401" => json_response(schema_ref("ErrorResponse"), "未认证")
	}));

	add_content_type_paths(paths);
	add_media_paths(paths);
	add_plugin_management_paths(paths);
	add_settings_paths(paths);
	add_user_role_paths(paths);
}

fn add_content_type_paths(paths: &mut Map<String, Value>) {
	insert_operation(paths, "/api/v1/content-types", "get", operation("列出内容类型", "content-types", true, None, None, map! {
		"200" => json_response(json!({ "type": "array", "items": schema_ref("ContentTypeDefinition") }), "内容类型列表")
	}));
	insert_operation(paths, "/api/v1/content-types", "post", operation("创建内容类型", "content-types", true, Some(json!({
		"required": true,
		"content": { "application/json": { "schema": schema_ref("ContentTypeMutation") } }
	})), None, map! {
		"201" => json_response(schema_ref("ContentTypeDefinition"), "创建成功")
	}));
	insert_operation(paths, "/api/v1/content-types/{api_id}", "get", operation("获取单个内容类型", "content-types", true, None, Some(vec![path_parameter("api_id", "string")]), map! {
		"200" => json_response(schema_ref("ContentTypeDefinition"), "内容类型详情"),
		"404" => json_response(schema_ref("ErrorResponse"), "未找到")
	}));
	insert_operation(paths, "/api/v1/content-types/{api_id}", "put", operation("更新内容类型", "content-types", true, Some(json!({
		"required": true,
		"content": { "application/json": { "schema": schema_ref("ContentTypePatch") } }
	})), Some(vec![path_parameter("api_id", "string")]), map! {
		"200" => json_response(schema_ref("ContentTypeDefinition"), "更新成功")
	}));
	insert_operation(paths, "/api/v1/content-types/{api_id}", "delete", operation("删除内容类型", "content-types", true, None, Some(vec![path_parameter("api_id", "string")]), map! {
		"204" => no_content_response("删除成功"),
		"404" => json_response(schema_ref("ErrorResponse"), "未找到")
	}));
}

fn add_dynamic_content_paths(paths: &mut Map<String, Value>, type_api_id: &str, entry_schema: &str) {
	let base = format!("/api/v1/content/{type_api_id}");
	let detail = format!("{base}/{{id}}");
	let revisions = format!("{detail}/revisions");
	let revision = format!("{revisions}/{{version}}");
	let rollback = format!("{revision}/rollback");
	let publish = format!("{detail}/publish");
	let unpublish = format!("{detail}/unpublish");

	insert_operation(paths, &base, "get", operation(&format!("列出 {type_api_id} 内容"), "content", true, None, Some(vec![query_parameter("page"), query_parameter("pageSize"), query_parameter("sort"), query_parameter("populate"), query_parameter("status")]), map! {
		"200" => json_response(json!({
			"type": "object",
			"required": ["data", "meta"],
			"properties": {
				"data": { "type": "array", "items": schema_ref(entry_schema) },
				"meta": schema_ref("PaginationMeta")
			}
		}), "内容分页列表")
	}));
	insert_operation(paths, &base, "post", operation(&format!("创建 {type_api_id} 内容"), "content", true, Some(json!({
		"required": true,
		"content": { "application/json": { "schema": {
			"type": "object",
			"required": ["data"],
			"properties": {
				"slug": nullable(json!({ "type": "string" })),
				"data": schema_ref(&format!("ContentFields{}", pascal_case(type_api_id)))
			}
		} } }
	})), None, map! {
		"201" => json_response(schema_ref(entry_schema), "创建成功")
	}));
	insert_operation(paths, &detail, "get", operation(&format!("获取单个 {type_api_id} 内容"), "content", true, None, Some(vec![path_parameter("id", "string"), query_parameter("populate")]), map! {
		"200" => json_response(schema_ref(entry_schema), "内容详情"),
		"404" => json_response(schema_ref("ErrorResponse"), "未找到")
	}));
	insert_operation(paths, &detail, "put", operation(&format!("更新 {type_api_id} 内容"), "content", true, Some(json!({
		"required": true,
		"content": { "application/json": { "schema": {
			"type": "object",
			"properties": {
				"slug": nullable(json!({ "type": "string" })),
				"data": schema_ref(&format!("ContentFields{}", pascal_case(type_api_id)))
			}
		} } }
	})), Some(vec![path_parameter("id", "string")]), map! {
		"200" => json_response(schema_ref(entry_schema), "更新成功")
	}));
	insert_operation(paths, &detail, "delete", operation(&format!("删除 {type_api_id} 内容"), "content", true, None, Some(vec![path_parameter("id", "string"), query_parameter("mode")]), map! {
		"204" => no_content_response("删除成功")
	}));
	insert_operation(paths, &publish, "post", operation(&format!("发布 {type_api_id} 内容"), "content", true, None, Some(vec![path_parameter("id", "string")]), map! {
		"200" => json_response(schema_ref(entry_schema), "发布成功")
	}));
	insert_operation(paths, &unpublish, "post", operation(&format!("撤回 {type_api_id} 内容"), "content", true, None, Some(vec![path_parameter("id", "string")]), map! {
		"200" => json_response(schema_ref(entry_schema), "撤回成功")
	}));
	insert_operation(paths, &revisions, "get", operation(&format!("列出 {type_api_id} 版本历史"), "content", true, None, Some(vec![path_parameter("id", "string"), query_parameter("page"), query_parameter("page_size")]), map! {
		"200" => json_response(schema_ref("RevisionListResponse"), "版本历史")
	}));
	insert_operation(paths, &revision, "get", operation(&format!("获取 {type_api_id} 指定版本"), "content", true, None, Some(vec![path_parameter("id", "string"), path_parameter("version", "integer")]), map! {
		"200" => json_response(schema_ref("Revision"), "版本详情")
	}));
	insert_operation(paths, &rollback, "post", operation(&format!("回滚 {type_api_id} 到指定版本"), "content", true, None, Some(vec![path_parameter("id", "string"), path_parameter("version", "integer")]), map! {
		"200" => json_response(schema_ref(entry_schema), "回滚成功")
	}));
}

fn add_media_paths(paths: &mut Map<String, Value>) {
	insert_operation(paths, "/api/v1/media", "get", operation("列出媒体", "media", true, None, Some(vec![query_parameter("page"), query_parameter("pageSize"), query_parameter("mime_type"), query_parameter("filename")]), map! {
		"200" => json_response(schema_ref("MediaListResponse"), "媒体列表")
	}));
	insert_operation(paths, "/api/v1/media/upload", "post", operation("上传媒体", "media", true, Some(json!({
		"required": true,
		"content": {
			"multipart/form-data": {
				"schema": {
					"type": "object",
					"required": ["file"],
					"properties": {
						"file": { "type": "string", "format": "binary" },
						"mime_type": { "type": "string" },
						"metadata": { "type": "string", "description": "JSON string" }
					}
				}
			}
		}
	})), None, map! {
		"201" => json_response(schema_ref("MediaAssetResponse"), "上传成功")
	}));
	insert_operation(paths, "/api/v1/media/{id}", "get", operation("获取媒体详情", "media", true, None, Some(vec![path_parameter("id", "string")]), map! {
		"200" => json_response(schema_ref("MediaAssetResponse"), "媒体详情")
	}));
	insert_operation(paths, "/api/v1/media/{id}", "delete", operation("删除媒体", "media", true, None, Some(vec![path_parameter("id", "string")]), map! {
		"204" => no_content_response("删除成功")
	}));
}

fn add_plugin_management_paths(paths: &mut Map<String, Value>) {
	insert_operation(paths, "/api/v1/plugins", "get", operation("列出插件", "plugins", true, None, None, map! {
		"200" => json_response(json!({ "type": "array", "items": schema_ref("PluginInfoResponse") }), "插件列表")
	}));
	insert_operation(paths, "/api/v1/plugins/install", "post", operation("安装插件", "plugins", true, Some(json!({
		"required": true,
		"content": { "application/json": { "schema": {
			"type": "object",
			"required": ["path"],
			"properties": { "path": { "type": "string" } }
		} } }
	})), None, map! {
		"201" => json_response(schema_ref("PluginInfoResponse"), "安装成功")
	}));
	insert_operation(paths, "/api/v1/plugins/{name}", "get", operation("获取插件详情", "plugins", true, None, Some(vec![path_parameter("name", "string")]), map! {
		"200" => json_response(schema_ref("PluginInfoResponse"), "插件详情")
	}));
	insert_operation(paths, "/api/v1/plugins/{name}", "delete", operation("卸载插件", "plugins", true, None, Some(vec![path_parameter("name", "string")]), map! {
		"204" => no_content_response("卸载成功")
	}));
	insert_operation(paths, "/api/v1/plugins/{name}/enable", "post", operation("启用插件", "plugins", true, None, Some(vec![path_parameter("name", "string")]), map! {
		"200" => json_response(schema_ref("PluginInfoResponse"), "启用成功")
	}));
	insert_operation(paths, "/api/v1/plugins/{name}/disable", "post", operation("停用插件", "plugins", true, Some(json!({
		"content": { "application/json": { "schema": {
			"type": "object",
			"properties": { "force": { "type": "boolean", "default": false } }
		} } }
	})), Some(vec![path_parameter("name", "string")]), map! {
		"200" => json_response(schema_ref("PluginInfoResponse"), "停用成功")
	}));
}

fn add_settings_paths(paths: &mut Map<String, Value>) {
	insert_operation(paths, "/api/v1/settings/schemas", "get", operation("列出插件设置 schema", "settings", true, None, None, map! {
		"200" => json_response(json!({ "type": "array", "items": schema_ref("PluginSchema") }), "设置 schema 列表")
	}));
	insert_operation(paths, "/api/v1/settings/{namespace}", "get", operation("列出命名空间设置", "settings", true, None, Some(vec![path_parameter("namespace", "string")]), map! {
		"200" => json_response(json!({ "type": "array", "items": schema_ref("SettingEntry") }), "设置列表")
	}));
	insert_operation(paths, "/api/v1/settings/{namespace}/{key}", "get", operation("读取设置项", "settings", true, None, Some(vec![path_parameter("namespace", "string"), path_parameter("key", "string")]), map! {
		"200" => json_response(schema_ref("SettingEntry"), "设置详情")
	}));
	insert_operation(paths, "/api/v1/settings/{namespace}/{key}", "put", operation("写入设置项", "settings", true, Some(json!({
		"required": true,
		"content": { "application/json": { "schema": {
			"type": "object",
			"required": ["value"],
			"properties": { "value": {} }
		} } }
	})), Some(vec![path_parameter("namespace", "string"), path_parameter("key", "string")]), map! {
		"200" => json_response(schema_ref("SettingEntry"), "写入成功")
	}));
	insert_operation(paths, "/api/v1/settings/{namespace}/{key}", "delete", operation("删除设置项", "settings", true, None, Some(vec![path_parameter("namespace", "string"), path_parameter("key", "string")]), map! {
		"204" => no_content_response("删除成功")
	}));
}

fn add_user_role_paths(paths: &mut Map<String, Value>) {
	insert_operation(paths, "/api/v1/users", "get", operation("列出用户", "users", true, None, None, map! {
		"200" => json_response(json!({ "type": "array", "items": schema_ref("UserResponse") }), "用户列表")
	}));
	insert_operation(paths, "/api/v1/users", "post", operation("创建用户", "users", true, Some(json!({
		"required": true,
		"content": { "application/json": { "schema": {
			"type": "object",
			"required": ["username", "email", "password"],
			"properties": {
				"username": { "type": "string" },
				"email": { "type": "string", "format": "email" },
				"password": { "type": "string", "format": "password" },
				"is_active": { "type": "boolean" },
				"role_ids": { "type": "array", "items": { "type": "string", "format": "uuid" } }
			}
		} } }
	})), None, map! {
		"201" => json_response(schema_ref("UserResponse"), "创建成功")
	}));
	insert_operation(paths, "/api/v1/users/{id}", "get", operation("获取用户详情", "users", true, None, Some(vec![path_parameter("id", "string")]), map! {
		"200" => json_response(schema_ref("UserResponse"), "用户详情")
	}));
	insert_operation(paths, "/api/v1/users/{id}", "put", operation("更新用户", "users", true, Some(json!({
		"required": true,
		"content": { "application/json": { "schema": {
			"type": "object",
			"properties": {
				"username": { "type": "string" },
				"email": { "type": "string", "format": "email" },
				"password": { "type": "string", "format": "password" },
				"is_active": { "type": "boolean" },
				"role_ids": { "type": "array", "items": { "type": "string", "format": "uuid" } }
			}
		} } }
	})), Some(vec![path_parameter("id", "string")]), map! {
		"200" => json_response(schema_ref("UserResponse"), "更新成功")
	}));
	insert_operation(paths, "/api/v1/users/{id}", "delete", operation("删除用户", "users", true, None, Some(vec![path_parameter("id", "string")]), map! {
		"204" => no_content_response("删除成功")
	}));

	insert_operation(paths, "/api/v1/roles", "get", operation("列出角色", "roles", true, None, None, map! {
		"200" => json_response(json!({ "type": "array", "items": schema_ref("RoleResponse") }), "角色列表")
	}));
	insert_operation(paths, "/api/v1/roles/permissions", "get", operation("列出权限目录", "roles", true, None, None, map! {
		"200" => json_response(json!({ "type": "array", "items": schema_ref("Permission") }), "权限目录")
	}));
	insert_operation(paths, "/api/v1/roles", "post", operation("创建角色", "roles", true, Some(json!({
		"required": true,
		"content": { "application/json": { "schema": {
			"type": "object",
			"required": ["name"],
			"properties": {
				"name": { "type": "string" },
				"description": nullable(json!({ "type": "string" })),
				"permission_ids": { "type": "array", "items": { "type": "string", "format": "uuid" } }
			}
		} } }
	})), None, map! {
		"201" => json_response(schema_ref("RoleResponse"), "创建成功")
	}));
	insert_operation(paths, "/api/v1/roles/{id}", "get", operation("获取角色详情", "roles", true, None, Some(vec![path_parameter("id", "string")]), map! {
		"200" => json_response(schema_ref("RoleResponse"), "角色详情")
	}));
	insert_operation(paths, "/api/v1/roles/{id}", "put", operation("更新角色", "roles", true, Some(json!({
		"required": true,
		"content": { "application/json": { "schema": {
			"type": "object",
			"properties": {
				"name": { "type": "string" },
				"description": nullable(json!({ "type": "string" })),
				"permission_ids": { "type": "array", "items": { "type": "string", "format": "uuid" } }
			}
		} } }
	})), Some(vec![path_parameter("id", "string")]), map! {
		"200" => json_response(schema_ref("RoleResponse"), "更新成功")
	}));
	insert_operation(paths, "/api/v1/roles/{id}", "delete", operation("删除角色", "roles", true, None, Some(vec![path_parameter("id", "string")]), map! {
		"204" => no_content_response("删除成功")
	}));
}

fn add_plugin_paths(
	paths: &mut Map<String, Value>,
	docs: Vec<(String, Vec<PluginRouteDoc>)>,
	route_plugins: Vec<String>,
) {
	let mut documented_plugins = HashSet::new();
	let grouped: HashMap<String, Vec<PluginRouteDoc>> = docs.into_iter().collect();

	for (plugin_name, route_docs) in grouped {
		documented_plugins.insert(plugin_name.clone());
		if route_docs.is_empty() {
			add_generic_plugin_path(paths, &plugin_name);
			continue;
		}
		for route in route_docs {
			let full_path = format!("/api/v1/x/{plugin_name}{}", normalize_plugin_path(&route.path));
			for method in route.methods {
				insert_operation(paths, &full_path, &method, operation(&format!("插件 {} 路由", plugin_name), "plugins", false, None, None, map! {
					"200" => json_response(json!({}), "插件响应"),
					"default" => json_response(schema_ref("ErrorResponse"), "错误")
				}));
			}
		}
	}

	for plugin_name in route_plugins {
		if !documented_plugins.contains(&plugin_name) {
			add_generic_plugin_path(paths, &plugin_name);
		}
	}
}

fn add_generic_plugin_path(paths: &mut Map<String, Value>, plugin_name: &str) {
	insert_operation(paths, &format!("/api/v1/x/{plugin_name}/{{path}}"), "get", operation(&format!("插件 {} 通配路由", plugin_name), "plugins", false, None, Some(vec![path_parameter("path", "string")]), map! {
		"200" => json_response(json!({}), "插件响应")
	}));
}

fn static_schemas() -> Map<String, Value> {
	let mut schemas = Map::<String, Value>::new();
	schemas.insert("ErrorResponse".to_owned(), json!({
		"type": "object",
		"required": ["error"],
		"properties": {
			"error": {
				"type": "object",
				"required": ["status", "name", "code", "message"],
				"properties": {
					"status": { "type": "integer" },
					"name": { "type": "string" },
					"code": { "type": "string" },
					"message": { "type": "string" },
					"details": {}
				}
			}
		}
	}));
	schemas.insert("TokenPairResponse".to_owned(), json!({
		"type": "object",
		"required": ["access_token", "refresh_token", "expires_in"],
		"properties": {
			"access_token": { "type": "string" },
			"refresh_token": { "type": "string" },
			"expires_in": { "type": "integer" }
		}
	}));
	schemas.insert("UserResponse".to_owned(), json!({
		"type": "object",
		"required": ["id", "username", "email", "is_active", "role_ids", "roles", "created_at", "updated_at"],
		"properties": {
			"id": { "type": "string", "format": "uuid" },
			"username": { "type": "string" },
			"email": { "type": "string", "format": "email" },
			"is_active": { "type": "boolean" },
			"role_ids": { "type": "array", "items": { "type": "string", "format": "uuid" } },
			"roles": { "type": "array", "items": { "type": "string" } },
			"created_at": { "type": "string", "format": "date-time" },
			"updated_at": { "type": "string", "format": "date-time" }
		}
	}));
	schemas.insert("Permission".to_owned(), json!({
		"type": "object",
		"required": ["id", "domain", "resource", "action", "scope", "source"],
		"properties": {
			"id": { "type": "string", "format": "uuid" },
			"domain": { "type": "string" },
			"resource": { "type": "string" },
			"action": { "type": "string" },
			"scope": { "type": "string", "enum": ["all", "own"] },
			"source": { "type": "string" }
		}
	}));
	schemas.insert("RoleResponse".to_owned(), json!({
		"type": "object",
		"required": ["id", "name", "is_system", "created_at", "permissions"],
		"properties": {
			"id": { "type": "string", "format": "uuid" },
			"name": { "type": "string" },
			"description": nullable(json!({ "type": "string" })),
			"is_system": { "type": "boolean" },
			"created_at": { "type": "string", "format": "date-time" },
			"permissions": { "type": "array", "items": schema_ref("Permission") }
		}
	}));
	schemas.insert("FieldDefinition".to_owned(), json!({
		"type": "object",
		"required": ["name", "api_id", "field_type", "required", "unique", "validations", "position"],
		"properties": {
			"name": { "type": "string" },
			"api_id": { "type": "string" },
			"field_type": { "type": "object", "additionalProperties": true },
			"required": { "type": "boolean" },
			"unique": { "type": "boolean" },
			"default_value": {},
			"validations": { "type": "array", "items": { "type": "object", "additionalProperties": true } },
			"position": { "type": "integer" }
		}
	}));
	schemas.insert("ContentTypeDefinition".to_owned(), json!({
		"type": "object",
		"required": ["id", "name", "api_id", "kind", "fields", "created_at", "updated_at"],
		"properties": {
			"id": { "type": "string", "format": "uuid" },
			"name": { "type": "string" },
			"api_id": { "type": "string" },
			"description": nullable(json!({ "type": "string" })),
			"kind": { "type": "string", "enum": ["collection", "single"] },
			"fields": { "type": "array", "items": schema_ref("FieldDefinition") },
			"created_at": { "type": "string", "format": "date-time" },
			"updated_at": { "type": "string", "format": "date-time" }
		}
	}));
	schemas.insert("ContentTypeMutation".to_owned(), json!({
		"type": "object",
		"required": ["name", "api_id", "kind", "fields"],
		"properties": {
			"name": { "type": "string" },
			"api_id": { "type": "string" },
			"description": nullable(json!({ "type": "string" })),
			"kind": { "type": "string", "enum": ["collection", "single"] },
			"fields": { "type": "array", "items": schema_ref("FieldDefinition") }
		}
	}));
	schemas.insert("ContentTypePatch".to_owned(), json!({
		"type": "object",
		"properties": {
			"name": { "type": "string" },
			"description": nullable(json!({ "type": "string" })),
			"kind": { "type": "string", "enum": ["collection", "single"] },
			"fields": { "type": "array", "items": schema_ref("FieldDefinition") }
		}
	}));
	schemas.insert("PaginationMeta".to_owned(), json!({
		"type": "object",
		"required": ["page", "page_size", "page_count", "total"],
		"properties": {
			"page": { "type": "integer" },
			"page_size": { "type": "integer" },
			"page_count": { "type": "integer" },
			"total": { "type": "integer" }
		}
	}));
	schemas.insert("MediaAssetResponse".to_owned(), json!({
		"type": "object",
		"required": ["id", "filename", "original_filename", "mime_type", "size", "storage_path", "uploaded_by", "created_at"],
		"properties": {
			"id": { "type": "string", "format": "uuid" },
			"filename": { "type": "string" },
			"original_filename": { "type": "string" },
			"mime_type": { "type": "string" },
			"size": { "type": "integer" },
			"storage_path": { "type": "string" },
			"metadata": {},
			"uploaded_by": { "type": "string", "format": "uuid" },
			"created_at": { "type": "string", "format": "date-time" }
		}
	}));
	schemas.insert("MediaListResponse".to_owned(), json!({
		"type": "object",
		"required": ["data", "total", "page", "page_size", "page_count"],
		"properties": {
			"data": { "type": "array", "items": schema_ref("MediaAssetResponse") },
			"total": { "type": "integer" },
			"page": { "type": "integer" },
			"page_size": { "type": "integer" },
			"page_count": { "type": "integer" }
		}
	}));
	schemas.insert("PluginInfoResponse".to_owned(), json!({
		"type": "object",
		"required": ["name", "version", "kind", "status", "dependencies", "permissions"],
		"properties": {
			"name": { "type": "string" },
			"version": { "type": "string" },
			"kind": { "type": "string" },
			"status": { "type": "string" },
			"dependencies": { "type": "array", "items": { "type": "string" } },
			"permissions": { "type": "array", "items": { "type": "string" } }
		}
	}));
	schemas.insert("SettingEntry".to_owned(), json!({
		"type": "object",
		"required": ["id", "namespace", "key", "value", "updated_at"],
		"properties": {
			"id": { "type": "string", "format": "uuid" },
			"namespace": { "type": "string" },
			"key": { "type": "string" },
			"value": {},
			"updated_at": { "type": "string", "format": "date-time" }
		}
	}));
	schemas.insert("PluginSchema".to_owned(), json!({
		"type": "object",
		"required": ["plugin_name", "schema", "created_at"],
		"properties": {
			"plugin_name": { "type": "string" },
			"schema": {},
			"created_at": { "type": "string", "format": "date-time" }
		}
	}));
	schemas.insert("Revision".to_owned(), json!({
		"type": "object",
		"required": ["id", "content_entry_id", "version_number", "snapshot", "created_by", "created_at"],
		"properties": {
			"id": { "type": "string", "format": "uuid" },
			"content_entry_id": { "type": "string", "format": "uuid" },
			"version_number": { "type": "integer" },
			"snapshot": {},
			"change_summary": nullable(json!({ "type": "string" })),
			"created_by": { "type": "string", "format": "uuid" },
			"created_at": { "type": "string", "format": "date-time" }
		}
	}));
	schemas.insert("RevisionListResponse".to_owned(), json!({
		"type": "object",
		"required": ["data", "total", "page", "page_size"],
		"properties": {
			"data": { "type": "array", "items": schema_ref("Revision") },
			"total": { "type": "integer" },
			"page": { "type": "integer" },
			"page_size": { "type": "integer" }
		}
	}));
	schemas
}

fn content_entry_schema(fields_schema_name: &str) -> Value {
	json!({
		"type": "object",
		"required": [
			"id",
			"content_type_id",
			"content_type_api_id",
			"status",
			"fields",
			"created_by",
			"updated_by",
			"created_at",
			"updated_at"
		],
		"properties": {
			"id": { "type": "string", "format": "uuid" },
			"content_type_id": { "type": "string", "format": "uuid" },
			"content_type_api_id": { "type": "string" },
			"slug": nullable(json!({ "type": "string" })),
			"status": { "type": "string", "enum": ["draft", "published", "archived"] },
			"current_version_id": nullable(json!({ "type": "string", "format": "uuid" })),
			"published_version_id": nullable(json!({ "type": "string", "format": "uuid" })),
			"fields": schema_ref(fields_schema_name),
			"created_by": { "type": "string", "format": "uuid" },
			"updated_by": { "type": "string", "format": "uuid" },
			"created_at": { "type": "string", "format": "date-time" },
			"updated_at": { "type": "string", "format": "date-time" },
			"published_at": nullable(json!({ "type": "string", "format": "date-time" })),
			"populated": nullable(json!({ "type": "object", "additionalProperties": true }))
		}
	})
}

fn nullable(schema: Value) -> Value {
	json!({
		"anyOf": [
			schema,
			{ "type": "null" }
		]
	})
}

fn operation(
	summary: &str,
	tag: &str,
	secured: bool,
	request_body: Option<Value>,
	parameters: Option<Vec<Value>>,
	responses: Map<String, Value>,
) -> Value {
	let mut operation = Map::<String, Value>::new();
	operation.insert("summary".to_owned(), Value::String(summary.to_owned()));
	operation.insert("tags".to_owned(), json!([tag]));
	if secured {
		operation.insert("security".to_owned(), json!([{"bearerAuth": []}]));
	}
	if let Some(request_body) = request_body {
		operation.insert("requestBody".to_owned(), request_body);
	}
	if let Some(parameters) = parameters {
		operation.insert("parameters".to_owned(), Value::Array(parameters));
	}
	operation.insert("responses".to_owned(), Value::Object(responses));
	Value::Object(operation)
}

fn insert_operation(paths: &mut Map<String, Value>, path: &str, method: &str, operation: Value) {
	let path_entry = paths
		.entry(path.to_owned())
		.or_insert_with(|| Value::Object(Map::new()));
	let object = path_entry.as_object_mut().expect("path entry must be object");
	object.insert(method.trim().to_ascii_lowercase(), operation);
}

fn json_response(schema: Value, description: &str) -> Value {
	json!({
		"description": description,
		"content": {
			"application/json": {
				"schema": schema
			}
		}
	})
}

fn no_content_response(description: &str) -> Value {
	json!({ "description": description })
}

fn schema_ref(name: &str) -> Value {
	json!({ "$ref": format!("#/components/schemas/{name}") })
}

fn path_parameter(name: &str, schema_type: &str) -> Value {
	json!({
		"name": name,
		"in": "path",
		"required": true,
		"schema": { "type": schema_type }
	})
}

fn query_parameter(name: &str) -> Value {
	json!({
		"name": name,
		"in": "query",
		"required": false,
		"schema": { "type": "string" }
	})
}

fn pascal_case(value: &str) -> String {
	value
		.split(['-', '_'])
		.filter(|segment| !segment.is_empty())
		.map(|segment| {
			let mut chars = segment.chars();
			match chars.next() {
				Some(first) => {
					let mut out = first.to_ascii_uppercase().to_string();
					out.push_str(chars.as_str());
					out
				}
				None => String::new(),
			}
		})
		.collect::<String>()
}

fn normalize_plugin_path(path: &str) -> String {
	if path.starts_with('/') {
		path.to_owned()
	} else {
		format!("/{path}")
	}
}

