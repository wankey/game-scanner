use crate::utils::from_rust;
use game_scanner::wegame;
use neon::prelude::*;

pub fn init(context: &mut ModuleContext) -> NeonResult<()> {
    let launcher = JsObject::new(context);
    let executable = JsFunction::new(context, executable)?;
    let find = JsFunction::new(context, find)?;
    let games = JsFunction::new(context, games)?;
    launcher.set(context, "executable", executable)?;
    launcher.set(context, "find", find)?;
    launcher.set(context, "games", games)?;
    context.export_value("wegame", launcher)
}

fn executable(mut context: FunctionContext) -> JsResult<JsString> {
    match wegame::executable() {
        Ok(path) => Ok(context.string(path.to_string_lossy())),
        Err(error) => context.throw_error(error.to_string()),
    }
}

fn find(mut context: FunctionContext) -> JsResult<JsObject> {
    let id = context.argument::<JsString>(0)?.value(&mut context);
    match wegame::find(&id) {
        Ok(game) => Ok(from_rust(&mut context, &game)),
        Err(error) => context.throw_error(error.to_string()),
    }
}

fn games(mut context: FunctionContext) -> JsResult<JsArray> {
    let games = match wegame::games() {
        Ok(games) => games,
        Err(error) => return context.throw_error(error.to_string()),
    };
    let array = JsArray::new(&mut context, games.len() as u32);
    for (i, game) in games.iter().enumerate() {
        let item = from_rust(&mut context, game);
        array.set(&mut context, i as u32, item)?;
    }
    Ok(array)
}
