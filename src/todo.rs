use crate::error_template::ErrorTemplate;
use leptos::{either::Either, prelude::*};
use leptos_meta::Stylesheet;
use serde::{Deserialize, Serialize};
use server_fn::ServerFnError;

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <AutoReload options=options.clone() />
                <HydrationScripts options/>
                <Stylesheet id="leptos" href="/pkg/todo-app.css"/>
                <link rel="shortcut icon" type="image/ico" href="/favicon.ico"/>
            </head>
            <body>
                <TodoApp/>
            </body>
        </html>
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct Todo {
    id: u16,
    title: String,
    completed: bool,
}

#[cfg(feature = "ssr")]
pub mod ssr {
    // use http::{header::SET_COOKIE, HeaderMap, HeaderValue, StatusCode};
    use leptos::server_fn::ServerFnError;
    use sqlx::{Connection, SqliteConnection};

    pub async fn db() -> Result<SqliteConnection, ServerFnError> {
        Ok(SqliteConnection::connect("sqlite:Todos.db").await?)
    }
}

#[server]
pub async fn get_todos() -> Result<Vec<Todo>, ServerFnError> {
    use self::ssr::*;
    use http::request::Parts;

    // this is just an example of how to access server context injected in the handlers
    let req_parts = use_context::<Parts>();

    if let Some(req_parts) = req_parts {
        println!("Uri = {:?}", req_parts.uri);
    }

    use futures::TryStreamExt;

    let mut conn = db().await?;

    let mut todos = Vec::new();
    let mut rows = sqlx::query_as::<_, Todo>("SELECT * FROM todos").fetch(&mut conn);
    while let Some(row) = rows.try_next().await? {
        todos.push(row);
    }

    // Lines below show how to set status code and headers on the response
    // let resp = expect_context::<ResponseOptions>();
    // resp.set_status(StatusCode::IM_A_TEAPOT);
    // resp.insert_header(SET_COOKIE, HeaderValue::from_str("fizz=buzz").unwrap());

    Ok(todos)
}

#[server]
pub async fn add_todo(title: String) -> Result<(), ServerFnError> {
    use self::ssr::*;
    let mut conn = db().await?;

    // fake API delay
    std::thread::sleep(std::time::Duration::from_millis(250));

    match sqlx::query("INSERT INTO todos (title, completed) VALUES ($1, false)")
        .bind(title)
        .execute(&mut conn)
        .await
    {
        Ok(_row) => Ok(()),
        Err(e) => Err(ServerFnError::ServerError(e.to_string())),
    }
}

#[server]
pub async fn delete_todo(id: u16) -> Result<(), ServerFnError> {
    use self::ssr::*;
    let mut conn = db().await?;

    Ok(sqlx::query("DELETE FROM todos WHERE id = $1")
        .bind(id)
        .execute(&mut conn)
        .await
        .map(|_| ())?)
}

#[component]
pub fn TodoApp() -> impl IntoView {
    view! {
        <main class="bg-gradient-to-tl from-orange-500 to-orange-300 text-white font-mono flex flex-col min-h-screen items-center justify-center">
        <div>
            <h1 class="text-2xl font-bold text-white mb-2">"My Tasks"</h1>
            <Todos/>
        </div>
        </main>
    }
}

#[component]
pub fn Todos() -> impl IntoView {
    let add_todo = ServerMultiAction::<AddTodo>::new();
    let submissions = add_todo.submissions();
    let delete_todo = ServerAction::<DeleteTodo>::new();

    // list of todos is loaded from the server in reaction to changes
    let todos = Resource::new(
        move || {
            (
                delete_todo.version().get(),
                add_todo.version().get(),
                delete_todo.version().get(),
            )
        },
        move |_| get_todos(),
    );

    let existing_todos = move || {
        Suspend::new(async move {
            todos
                .await
                .map(|todos| {
                    if todos.is_empty() {
                        Either::Left(view! { <p>"No tasks were found."</p> })
                    } else {
                        Either::Right(
                            todos
                                .iter()
                                .map(move |todo| {
                                    let id = todo.id;
                                    view! {
                                        <li class="flex flex-row gap-2 mb-2 items-center">
                                            <div class="font-medium"> {todo.title.clone()} </div>
                                            <ActionForm action=delete_todo>
                                                <input type="hidden" name="id" value=id/>
                                                <input type="submit" value="X"  class="rounded px-1 py-1 m-1 border-b-4 border-l-2 shadow-lg bg-orange-400 border-orange-500 text-white"/>
                                            </ActionForm>
                                        </li>
                                    }
                                })
                                .collect::<Vec<_>>(),
                        )
                    }
                })
        })
    };

    view! {
        <MultiActionForm action=add_todo>
        <div class="flex flex-row mb-2 gap-2 items-center">
            <label class="text-xl font-semibold text-white mb-2">"Add a Todo" <input type="text" name="title" class="bg-gray-50 border border-gray-300 text-gray-900 text-sm rounded-lg focus:ring-blue-500 focus:border-blue-500 block w-auto p-2.5 dark:bg-gray-700 dark:border-gray-600 dark:placeholder-gray-400 dark:text-white dark:focus:ring-blue-500 dark:focus:border-blue-500" placeholder="Type here..." required /></label>
            <input type="submit" value="Add" />
            </div>
        </MultiActionForm>
        <div>
            <Transition fallback=move || view! { <p>"Loading..."</p> }>
                <ErrorBoundary fallback=|errors| view! { <ErrorTemplate errors/> }>
                    <ul>
                        {existing_todos}
                        {move || {
                            submissions
                                .get()
                                .into_iter()
                                .filter(|submission| submission.pending().get())
                                .map(|submission| {
                                    view! {
                                        <li class="pending">
                                            {move || submission.input().get().map(|data| data.title)}
                                        </li>
                                    }
                                })
                                .collect::<Vec<_>>()
                        }}

                    </ul>
                </ErrorBoundary>
            </Transition>
        </div>
    }
}