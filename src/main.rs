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
        // println!("{} {}", prefix, command);
        if prefix == CMD_PREFIX {
            let help_msg = "Command list: \n\
                        \t`allbut`: Displays all messages that don't match the input.\n\t\tExample: `/s allbut Melee HD`\n\
                        \t`and`: Displays all messages that include every comma-separated term.\n\t\tExample: `/s and dogs, cats, pigs and bats`\n\
                        \t`exact`: Displays all messages that include the exact term entered.\n\t\tExample: `/s exact Specifically this`\n\
                        \t`or`: Displays all messages that include one or more of the comma-separated terms.\n\t\tExample: `/s or cats, dogs, pigs, bats`";
            match command {
                "ping" => send_message(&ctx, msg.channel_id, "Pong!").await,

                "allbut" => show_results(&ctx, msg.channel_id, allbut(&ctx, msg).await).await,

                "and" => show_results(&ctx, msg.channel_id, and_match(&ctx, msg).await).await,

                "exact" => show_results(&ctx, msg.channel_id, exact_match(&ctx, msg).await).await,

                "or" => show_results(&ctx, msg.channel_id, or_match(&ctx, msg).await).await,

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

// Helper function to send safer messages
async fn send_message(ctx: &Context, channel: ChannelId, message: &str) {
    // Sending a message can fail, due to a network error, an
    // authentication error, or lack of permissions to post in the
    // channel, so log to stdout when some error happens, with a
    // description of it.
    if let Err(why) = channel.say(&ctx.http, message).await {
        println!("Error sending message: {:?}", why);
    }
}

// Display results in an embedded message (or messages, if needed)
async fn show_results(ctx: &Context, channel: ChannelId, results: Vec<Message>) {
    let mut result_fields: Vec<(String, String, bool)> = Vec::new();

    for result in results {
        let metadata = format!("{} at {}:", result.author.name, result.timestamp.date());
        let message_length = cmp::min(result.content.len(), 200);
        let message_str = &result.content[0..message_length];
        let message = message_str.to_string();

        result_fields.push((metadata, message, true));
    }

    let mut num_messages = result_fields.len() / 25;
    if result_fields.len() % 25 > 0 { num_messages += 1 }

    let mut input_fields: Vec<_>;

    for i in 0..num_messages {
        input_fields = result_fields.to_owned().iter().cloned()
                                    .skip(i*25).take(25).collect();
        let msg = channel.send_message(&ctx, |m| {
            m.content(format!("Results {}/{}:", i+1, num_messages));
            m.embed(|e| {
                e.fields(input_fields);
                e
            });
            m
        }).await;

        // println!("{:?}", msg);
        if let Err(why) = msg {
            println!("Error sending message: {:?}", why);
        }
    }
}

// Base search function that loops through messages and checks criteria
async fn search<F>(ctx: &Context, msg: Message, criteria: F) -> Vec<Message> where F: Fn(String, &str) -> bool {
    let msg_content: Vec<&str> = msg.content.splitn(3, " ").collect();
    let search = msg_content[2];
    let mut messages = get_100_messages(&ctx, msg.channel_id, msg.id).await;
    messages.reverse();
    let mut results: Vec<Message> = Vec::new();

    while messages.len() > 0 {
        // println!("{:?}", messages);
        let message = messages.pop().unwrap();
        let content = message.content.to_owned();
        let message_id = message.id;
        if criteria(content, search) {
            results.push(message)
        }
        if messages.len() == 0 {
            messages = get_100_messages(&ctx, msg.channel_id, message_id).await;
            messages.reverse()
        }
    }

    return results
}

// Searches all messages in the given channel and returns any that don't exactly match the result 
async fn allbut(ctx: &Context, msg: Message) -> Vec<Message> {
    return search(ctx, msg, |message, search| {
        message != search
    }).await
}

// Returns messages that contain an exact match for the input
async fn exact_match(ctx: &Context, msg: Message) -> Vec<Message> {
    return search(ctx, msg, |message, search| {
        message.contains(search)
    }).await
}

// Returns messages that contain two exact terms
async fn and_match(ctx: &Context, msg: Message) -> Vec<Message> {
    return search(ctx, msg, |message, search| {
        let searches: Vec<&str> = search.split_terminator(",").collect();
        let mut contains_all = true;

        for term in searches {
            if !message.contains(term) {
                contains_all = false;
            }
        }

        contains_all
    }).await
}

// Returns messages that contain one or more specified terms
async fn or_match(ctx: &Context, msg: Message) -> Vec<Message> {
    return search(ctx, msg, |message, search| {
        let searches: Vec<&str> = search.split_terminator(",").collect();
        let mut contains_any = false;

        for term in searches {
            if message.contains(term) {
                contains_any = true;
            }
        }

        contains_any
    }).await
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
