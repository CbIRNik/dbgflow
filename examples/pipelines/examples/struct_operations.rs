//! Example: Complex Struct Operations
//!
//! Demonstrates tracing operations on complex nested structs,
//! showing state transformations and field updates.

use dbgflow::prelude::*;

// ============================================================================
// Domain models
// ============================================================================

#[ui_debug(name = "User Account")]
struct UserAccount {
    id: u64,
    username: String,
    email: String,
    profile: UserProfile,
    settings: AccountSettings,
    status: AccountStatus,
}

#[ui_debug(name = "User Profile")]
struct UserProfile {
    display_name: String,
    bio: Option<String>,
    avatar_url: Option<String>,
    location: Option<String>,
    joined_at: String,
    follower_count: u32,
    following_count: u32,
}

#[ui_debug(name = "Account Settings")]
struct AccountSettings {
    theme: Theme,
    notifications: NotificationSettings,
    privacy: PrivacySettings,
    language: String,
}

#[ui_debug(name = "Theme")]
enum Theme {
    Light,
    Dark,
    System,
}

#[ui_debug(name = "Notification Settings")]
struct NotificationSettings {
    email_enabled: bool,
    push_enabled: bool,
    digest_frequency: DigestFrequency,
}

#[ui_debug(name = "Digest Frequency")]
enum DigestFrequency {
    Daily,
    Weekly,
    Never,
}

#[ui_debug(name = "Privacy Settings")]
struct PrivacySettings {
    profile_visible: bool,
    show_email: bool,
    allow_mentions: bool,
}

#[ui_debug(name = "Account Status")]
enum AccountStatus {
    Active,
    Suspended { reason: String, until: Option<String> },
    Deactivated,
    PendingVerification,
}

// ============================================================================
// Builder pattern
// ============================================================================

#[ui_debug(name = "Account Builder")]
struct AccountBuilder {
    id: Option<u64>,
    username: Option<String>,
    email: Option<String>,
    display_name: Option<String>,
    build_step: String,
}

impl AccountBuilder {
    fn new() -> Self {
        Self {
            id: None,
            username: None,
            email: None,
            display_name: None,
            build_step: "initialized".to_owned(),
        }
    }
}

#[trace(name = "Set Account ID")]
fn builder_with_id(builder: &mut AccountBuilder, id: u64) {
    builder.id = Some(id);
    builder.build_step = "id set".to_owned();
    builder.emit_snapshot("id configured");
}

#[trace(name = "Set Username")]
fn builder_with_username(builder: &mut AccountBuilder, username: &str) {
    builder.username = Some(username.to_owned());
    builder.build_step = "username set".to_owned();
    builder.emit_snapshot("username configured");
}

#[trace(name = "Set Email")]
fn builder_with_email(builder: &mut AccountBuilder, email: &str) {
    builder.email = Some(email.to_owned());
    builder.build_step = "email set".to_owned();
    builder.emit_snapshot("email configured");
}

#[trace(name = "Set Display Name")]
fn builder_with_display_name(builder: &mut AccountBuilder, name: &str) {
    builder.display_name = Some(name.to_owned());
    builder.build_step = "display name set".to_owned();
    builder.emit_snapshot("display name configured");
}

#[trace(name = "Build Account")]
fn build_account(builder: &AccountBuilder) -> UserAccount {
    builder.emit_snapshot("building final account");

    UserAccount {
        id: builder.id.unwrap_or(0),
        username: builder.username.clone().unwrap_or_default(),
        email: builder.email.clone().unwrap_or_default(),
        profile: UserProfile {
            display_name: builder.display_name.clone().unwrap_or_else(|| {
                builder.username.clone().unwrap_or_default()
            }),
            bio: None,
            avatar_url: None,
            location: None,
            joined_at: "2024-01-15".to_owned(),
            follower_count: 0,
            following_count: 0,
        },
        settings: AccountSettings {
            theme: Theme::System,
            notifications: NotificationSettings {
                email_enabled: true,
                push_enabled: true,
                digest_frequency: DigestFrequency::Weekly,
            },
            privacy: PrivacySettings {
                profile_visible: true,
                show_email: false,
                allow_mentions: true,
            },
            language: "en".to_owned(),
        },
        status: AccountStatus::PendingVerification,
    }
}

// ============================================================================
// Account operations
// ============================================================================

#[trace(name = "Verify Account")]
fn verify_account(account: &mut UserAccount) {
    account.emit_snapshot("starting verification");
    account.status = AccountStatus::Active;
    account.emit_snapshot("account verified and active");
}

#[trace(name = "Update Profile")]
fn update_profile(account: &mut UserAccount, bio: &str, location: &str) {
    account.emit_snapshot("updating profile fields");

    account.profile.bio = Some(bio.to_owned());
    account.profile.location = Some(location.to_owned());

    account.emit_snapshot("profile updated");
}

#[trace(name = "Change Theme")]
fn change_theme(account: &mut UserAccount, theme: Theme) {
    account.emit_snapshot(&format!("current theme: {:?}", account.settings.theme));
    account.settings.theme = theme;
    account.emit_snapshot("theme updated");
}

#[trace(name = "Configure Notifications")]
fn configure_notifications(account: &mut UserAccount, email: bool, push: bool, digest: DigestFrequency) {
    account.emit_snapshot("configuring notification preferences");

    account.settings.notifications.email_enabled = email;
    account.settings.notifications.push_enabled = push;
    account.settings.notifications.digest_frequency = digest;

    account.emit_snapshot("notifications configured");
}

#[trace(name = "Update Privacy")]
fn update_privacy(account: &mut UserAccount, profile_visible: bool, show_email: bool) {
    account.emit_snapshot("updating privacy settings");

    account.settings.privacy.profile_visible = profile_visible;
    account.settings.privacy.show_email = show_email;

    account.emit_snapshot("privacy settings updated");
}

#[trace(name = "Add Followers")]
fn simulate_followers(account: &mut UserAccount, new_followers: u32, new_following: u32) {
    account.emit_snapshot(&format!("current: {} followers, {} following",
                                    account.profile.follower_count,
                                    account.profile.following_count));

    account.profile.follower_count += new_followers;
    account.profile.following_count += new_following;

    account.emit_snapshot(&format!("updated: {} followers, {} following",
                                    account.profile.follower_count,
                                    account.profile.following_count));
}

#[trace(name = "Suspend Account")]
fn suspend_account(account: &mut UserAccount, reason: &str) {
    account.emit_snapshot("initiating suspension");

    account.status = AccountStatus::Suspended {
        reason: reason.to_owned(),
        until: Some("2024-02-15".to_owned()),
    };

    account.emit_snapshot("account suspended");
}

#[trace(name = "Reactivate Account")]
fn reactivate_account(account: &mut UserAccount) {
    account.emit_snapshot("reactivating account");
    account.status = AccountStatus::Active;
    account.emit_snapshot("account reactivated");
}

// ============================================================================
// Complex nested updates
// ============================================================================

#[ui_debug(name = "Order")]
struct Order {
    id: u64,
    customer_id: u64,
    items: Vec<OrderItem>,
    shipping: ShippingInfo,
    status: OrderStatus,
    totals: OrderTotals,
}

#[ui_debug(name = "Order Item")]
struct OrderItem {
    product_id: u64,
    name: String,
    quantity: u32,
    unit_price: f64,
}

#[ui_debug(name = "Shipping Info")]
struct ShippingInfo {
    address: Address,
    method: ShippingMethod,
    tracking_number: Option<String>,
}

#[ui_debug(name = "Address")]
struct Address {
    street: String,
    city: String,
    state: String,
    zip: String,
    country: String,
}

#[ui_debug(name = "Shipping Method")]
enum ShippingMethod {
    Standard,
    Express,
    Overnight,
}

#[ui_debug(name = "Order Status")]
enum OrderStatus {
    Pending,
    Processing,
    Shipped,
    Delivered,
    Cancelled,
}

#[ui_debug(name = "Order Totals")]
struct OrderTotals {
    subtotal: f64,
    shipping: f64,
    tax: f64,
    total: f64,
}

#[trace(name = "Create Order")]
fn create_order(customer_id: u64) -> Order {
    let order = Order {
        id: 1001,
        customer_id,
        items: Vec::new(),
        shipping: ShippingInfo {
            address: Address {
                street: "123 Main St".to_owned(),
                city: "Springfield".to_owned(),
                state: "IL".to_owned(),
                zip: "62701".to_owned(),
                country: "USA".to_owned(),
            },
            method: ShippingMethod::Standard,
            tracking_number: None,
        },
        status: OrderStatus::Pending,
        totals: OrderTotals {
            subtotal: 0.0,
            shipping: 0.0,
            tax: 0.0,
            total: 0.0,
        },
    };
    order.emit_snapshot("order created");
    order
}

#[trace(name = "Add Item")]
fn add_item(order: &mut Order, product_id: u64, name: &str, quantity: u32, price: f64) {
    order.emit_snapshot(&format!("adding {} x {}", quantity, name));

    order.items.push(OrderItem {
        product_id,
        name: name.to_owned(),
        quantity,
        unit_price: price,
    });

    recalculate_totals(order);
    order.emit_snapshot("item added");
}

#[trace(name = "Recalculate Totals")]
fn recalculate_totals(order: &mut Order) {
    order.totals.subtotal = order.items.iter()
        .map(|item| item.unit_price * item.quantity as f64)
        .sum();

    order.totals.shipping = match order.shipping.method {
        ShippingMethod::Standard => 5.99,
        ShippingMethod::Express => 12.99,
        ShippingMethod::Overnight => 24.99,
    };

    order.totals.tax = order.totals.subtotal * 0.08;
    order.totals.total = order.totals.subtotal + order.totals.shipping + order.totals.tax;

    order.emit_snapshot(&format!("totals: ${:.2}", order.totals.total));
}

#[trace(name = "Process Order")]
fn process_order(order: &mut Order) {
    order.emit_snapshot("processing order");
    order.status = OrderStatus::Processing;
    order.emit_snapshot("order processing");
}

#[trace(name = "Ship Order")]
fn ship_order(order: &mut Order, tracking: &str) {
    order.emit_snapshot("preparing shipment");
    order.status = OrderStatus::Shipped;
    order.shipping.tracking_number = Some(tracking.to_owned());
    order.emit_snapshot(&format!("shipped with tracking: {}", tracking));
}

// ============================================================================
// Main pipeline
// ============================================================================

#[trace(name = "Run Struct Examples")]
fn run_examples() {
    // Builder pattern example
    let mut builder = AccountBuilder::new();
    builder_with_id(&mut builder, 12345);
    builder_with_username(&mut builder, "johndoe");
    builder_with_email(&mut builder, "john@example.com");
    builder_with_display_name(&mut builder, "John Doe");

    let mut account = build_account(&builder);
    println!("Created account for: {}", account.username);

    // Account operations
    verify_account(&mut account);
    update_profile(&mut account, "Software developer and open source enthusiast", "San Francisco, CA");
    change_theme(&mut account, Theme::Dark);
    configure_notifications(&mut account, true, false, DigestFrequency::Daily);
    update_privacy(&mut account, true, false);
    simulate_followers(&mut account, 150, 42);

    println!("Account verified with {} followers", account.profile.follower_count);

    // Suspension and reactivation
    suspend_account(&mut account, "Terms of service violation");
    reactivate_account(&mut account);

    // Order processing example
    let mut order = create_order(account.id);
    add_item(&mut order, 101, "Rust Programming Book", 1, 49.99);
    add_item(&mut order, 102, "Mechanical Keyboard", 1, 149.99);
    add_item(&mut order, 103, "USB-C Cable", 3, 9.99);

    process_order(&mut order);
    ship_order(&mut order, "1Z999AA10123456784");

    println!("Order total: ${:.2}", order.totals.total);
    println!("Tracking: {:?}", order.shipping.tracking_number);
}

fn main() -> std::io::Result<()> {
    dbgflow::capture_to_file(
        "Struct Operations Pipeline",
        "struct_operations_session.json",
        run_examples,
    )?;

    println!("\nSession saved to struct_operations_session.json");
    Ok(())
}
