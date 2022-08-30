use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount, Transfer};

declare_id!("2bmGLNZ1dXFiWcS7x8DffzQVHUuPhhJ4vRKLw2SSdBNB");

const VENUE_SEED: &'static [u8] = b"venue";
const TICKET_SEED: &'static [u8] = b"ticket";

#[program]
pub mod concert_ticket {
    use super::*;

    pub fn create_venue(ctx: Context<CreateVenue>, venue_id: String) -> Result<()> {
        let venue = &mut ctx.accounts.venue_account;

        venue.owner = ctx.accounts.authority.key();
        venue.available_tickets = Vec::new();
        venue.token_mint = ctx.accounts.token_mint.key();
        venue.owner_token_account = ctx.accounts.token_account.key();

        Ok(())
    }

    pub fn create_tickets(
        ctx: Context<CreateTicket>,
        venue_id: String,
        venue_bump: u8,
        ticket_name: String,
        price: u64,
        available_tickets: u64,
    ) -> Result<()> {
        let venue = &mut ctx.accounts.venue_account;

        let new_tickets = Ticket {
            name: ticket_name,
            price: price,
            available: available_tickets,
        };

        venue.available_tickets.push(new_tickets);

        Ok(())
    }

    pub fn purchase_tickets(
        ctx: Context<PurchaseTicket>,
        venue_id: String,
        venue_bump: u8,
        ticket_name: String,
        quantity: u64,
    ) -> Result<()> {
        let venue = &mut ctx.accounts.venue_account;
        let mut index = usize::MAX;

        for i in 0..venue.available_tickets.len() {
            if venue.available_tickets[i].name == ticket_name {
                index = i;
                break;
            }
        }

        if index == usize::MAX {
            return err!(ErrorCode::InvalidTicketName);
        }

        let mut tickets = venue.available_tickets[index].clone();

        if tickets.available < quantity {
            return err!(ErrorCode::TicketsNotAvailable);
        }

        let bump_vector = venue_bump.to_le_bytes();
        let inner = vec![
            VENUE_SEED,
            venue_id.as_ref(),
            bump_vector.as_ref()
        ];

        let outer = vec![inner.as_slice()];

        let transfer_instruction = Transfer{
            from: ctx.accounts.buyer_token_account.to_account_info(),
            to: ctx.accounts.venue_owner_token_account.to_account_info(),
            authority: ctx.accounts.buyer.to_account_info(),
        };

        let cpi_ctx = CpiContext::new_with_signer(ctx.accounts.token_program.to_account_info(),transfer_instruction, outer.as_slice());
        anchor_spl::token::transfer(cpi_ctx, tickets.price)?;

        tickets.available = tickets.available.checked_sub(quantity).ok_or_else(|| ErrorCode::InvalidSub).unwrap();

        let purchased_tickets = &mut ctx.accounts.buyer_account;

        purchased_tickets.ticket = tickets;
        purchased_tickets.quantity = quantity;
        purchased_tickets.date_of_purchase = Clock::get().unwrap().unix_timestamp;

        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(venue_id: String)]
pub struct CreateVenue<'info> {
    #[account(init, payer = authority, seeds = [VENUE_SEED, venue_id.as_ref()], bump, space = 1000)]
    pub venue_account: Account<'info, Venue>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub token_mint: Account<'info, Mint>,
    #[account(mut, constraint = token_account.mint == token_mint.key())]
    pub token_account: Account<'info, TokenAccount>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(venue_id: String, venue_bump: u8)]
pub struct CreateTicket<'info> {
    #[account(mut, seeds = [VENUE_SEED, venue_id.as_ref()], bump = venue_bump, has_one = owner)]
    pub venue_account: Account<'info, Venue>,
    #[account(mut)]
    pub owner: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(venue_id: String, venue_bump: u8)]
pub struct PurchaseTicket<'info> {
    #[account(mut, seeds = [VENUE_SEED, venue_id.as_ref()], bump = venue_bump)]
    pub venue_account: Account<'info, Venue>,
    #[account(init, payer = buyer, seeds = [TICKET_SEED, venue_id.as_ref(), buyer.key().as_ref()], bump, space = 100)]
    pub buyer_account: Account<'info, PurchasedTickets>,
    #[account(mut)]
    pub buyer: Signer<'info>,
    #[account(mut)]
    pub buyer_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub venue_owner_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
    pub system_program: Program<'info, System>,
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, PartialEq)]
pub struct Ticket {
    pub name: String,
    pub price: u64,
    pub available: u64,
}

#[account]
pub struct Venue {
    pub owner: Pubkey,
    pub available_tickets: Vec<Ticket>,
    pub token_mint: Pubkey,
    pub owner_token_account: Pubkey,
}

#[account]
pub struct PurchasedTickets {
    pub ticket: Ticket,
    pub quantity: u64,
    pub date_of_purchase: i64,
}

#[error_code]
pub enum ErrorCode {
    #[msg("The ticket name doesnt exists, please provide a valid ticket name")]
    InvalidTicketName,
    #[msg("Tickets are over, they are not available")]
    TicketsNotAvailable,
    #[msg("Underflow")]
    InvalidSub,
}
