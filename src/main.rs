extern crate dotenv;

use dotenv::dotenv;
use std::{env, cmp};

use serenity::{
    async_trait,
    model::{channel::Message, gateway::Ready, id::{ChannelId, MessageId}},
    prelude::*,
};

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    // Set a handler for the `message` event - so that whenever a new message
    // is received - the closure (or function) passed will be called.
    //
    // Event handlers are dispatched through a threadpool, and so multiple
    // events can be dispatched simultaneously.
    async fn message(&self, ctx: Context, msg: Message) {
        static CMD_PREFIX: &str = "/s";

        let mut iter = msg.content.splitn(3, " ");
        // println!("{:?}", iter);
        let prefix = iter.next().unwrap();
        let command = iter.next().unwrap();
        // let _content = iter.next().unwrap();
        println!("{} {}", prefix, command);
        if prefix == CMD_PREFIX {
            let help_msg = "Command list: \n\
                        \t`ping`: Prints \"pong\"\n\
                        \t`allbut`: Displays all messages that don't match the input.\n\t\tExample: `/s allbut Melee HD`";
            match command {
                "ping" => send_message(&ctx, msg.channel_id, "Pong!").await,

                "allbut" => show_results(&ctx, msg.channel_id, allbut(&ctx, msg).await).await,

                "help" => send_message(&ctx, msg.channel_id, help_msg).await,

                _ => send_message(&ctx, msg.channel_id, "Command not recognized. Try `/s help` instead.").await,
            }
        }
    }

    // Set a handler to be called on the `ready` event. This is called when a
    // shard is booted, and a READY payload is sent by Discord. This payload
    // contains data like the current user's guild Ids, current user data,
    // private channels, and more.
    //
    // In this case, just print what the current user's username is.
    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}


async fn send_message(ctx: &Context, channel: ChannelId, message: &str) {
    // Sending a message can fail, due to a network error, an
    // authentication error, or lack of permissions to post in the
    // channel, so log to stdout when some error happens, with a
    // description of it.
    if let Err(why) = channel.say(&ctx.http, message).await {
        println!("Error sending message: {:?}", why);
    }
}

async fn show_results(ctx: &Context, channel: ChannelId, results: Vec<Message>) {
    let mut fields: Vec<(String, String, bool)> = Vec::new();

    for result in results {
        let metadata = format!("{} at {}:", result.author.name, result.timestamp.date());
        let message_length = cmp::min(result.content.len(), 200);
        let message_str = &result.content[0..message_length];
        let message = message_str.to_string();

        fields.push((metadata, message, true));
    }

    let msg = channel.send_message(&ctx, |m| {
        m.content("Results:");
        m.embed(|e| {
            e.fields(fields);
            e
        });
        m
    }).await;

    println!("{:?}", msg);
    if let Err(why) = msg {
        println!("Error sending message: {:?}", why);
    }
}


// Searches all messages in the given channel and returns any that don't exactly match the result 
async fn allbut(ctx: &Context, msg: Message) -> Vec<Message> {
    let msg_content: Vec<&str> = msg.content.splitn(3, " ").collect();
    let search = msg_content[2];
    let mut messages = get_100_messages(&ctx, msg.channel_id, msg.id).await;
    messages.reverse();
    let mut results: Vec<Message> = Vec::new();

    while messages.len() > 0 {
        println!("{:?}", messages);
        let message = messages.pop().unwrap();
        let message_id = message.id;
        if message.content != search {
            results.push(message)
        }
        if messages.len() == 0 {
            messages = get_100_messages(&ctx, msg.channel_id, message_id).await;
            messages.reverse()
        }
    }

    println!("{:?}", results);
    return results
}

// Returns an array of 100 messages and the last message's id
async fn get_100_messages(ctx: &Context, channel: ChannelId, msg_id: MessageId) -> Vec<Message> {
    let messages = channel.messages(&ctx.http, |retriever| {
        retriever.before(msg_id).limit(100)
    }).await;

    // println!("{:?}", messages)
    return messages.unwrap();
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    // Configure the client with your Discord bot token in the environment.
    let token = env::var("DISCORD_TOKEN")
        .expect("Expected a token in the environment");

    // Create a new instance of the Client, logging in as a bot. This will
    // automatically prepend your bot token with "Bot ", which is a requirement
    // by Discord for bot users.
    let mut client = Client::builder(&token)
        .event_handler(Handler)
        .await
        .expect("Err creating client");

    // Finally, start a single shard, and start listening to events.
    //
    // Shards will automatically attempt to reconnect, and will perform
    // exponential backoff until it reconnects.
    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}
